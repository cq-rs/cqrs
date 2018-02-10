extern crate okazis;
extern crate okazis_memory;
extern crate fnv;

use okazis::{Since, EventStore, StateStore, PersistedSnapshot, Precondition};
use okazis_memory::{MemoryEventStore, MemoryStateStore};

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
enum Event {
    Added(usize),
    Subtracted(usize),
    Multiplied(usize),
    DividedBy(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
enum Command {
    Add(usize),
    Subtract(usize),
    Multiply(usize),
    DivideBy(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
enum CommandError {
    DivideByZero,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
struct State {
    value: usize
}

impl Default for State {
    fn default() -> Self {
        State { value: 0 }
    }
}

impl State {
    fn apply(&self, event: Event) -> Self {
        match event {
            Event::Added(x) => State { value: self.value + x },
            Event::Subtracted(x) => State { value: self.value - x },
            Event::Multiplied(x) => State { value: self.value * x },
            Event::DividedBy(x) => State { value: self.value / x },
        }
    }

    fn apply_mut(&mut self, event: Event) {
        let new_state = self.apply(event);
        let _ = ::std::mem::replace(self, new_state);
    }

    fn execute(&self, cmd: Command) -> Result<Vec<Event>, CommandError> {
        let result =
            match cmd {
                Command::Add(x) => self.value.checked_add(x).map(|_| Event::Added(x)),
                Command::Subtract(x) => self.value.checked_sub(x).map(|_| Event::Subtracted(x)),
                Command::Multiply(x) => self.value.checked_mul(x).map(|_| Event::Multiplied(x)),
                Command::DivideBy(x) => {
                    if x == 0 {
                        return Err(CommandError::DivideByZero);
                    }
                    self.value.checked_div(x).map(|_| Event::DividedBy(x))
                }
            };
        result.map(|x| vec![x]).ok_or(CommandError::Overflow)
    }
}

fn main() {
    let events = vec![
        Event::Added(100),
        Event::Subtracted(36),
        Event::Multiplied(4),
        Event::DividedBy(128),
    ];

    let state = events.iter().fold(State::default(), |s, e| s.apply(e.clone()));
    assert_eq!(state, State { value: 2 });

    let mut mut_state = State::default();
    for event in &events {
        mut_state.apply_mut(event.clone());
    }

    assert_eq!(mut_state, State { value: 2 });

    let es = MemoryEventStore::<_, _, _, fnv::FnvBuildHasher>::default();
    {
        let result = es.append_events(&0, &vec![
            (Event::Added(100), ()),
            (Event::Subtracted(36), ()),
        ], Precondition::Always);
        assert!(result.is_ok());
    }
    {
        let past_events = es.read(&0, Since::BeginningOfStream).unwrap().unwrap();
        let state = past_events.iter().fold(State::default(), |s, e| s.apply(e.event));

        let result = state.execute(Command::Multiply(-1isize as usize));
        assert_eq!(result, Err(CommandError::Overflow));

        let result = state.execute(Command::DivideBy(0));
        assert_eq!(result, Err(CommandError::DivideByZero));

        let result = state.execute(Command::Multiply(4));
        assert_eq!(result, Ok(vec![Event::Multiplied(4)]));

        let decorated_events: Vec<_> = result.unwrap().into_iter()
            .map(|e| (e, ()))
            .collect();

        let result = es.append_events(&0, &decorated_events, Precondition::Always);
        assert!(result.is_ok());
    }
    {
        let new_events = es.read(&0, Since::Offset(0)).unwrap().unwrap();
        let state = new_events.iter().fold(State { value: 36 }, |s, e| s.apply(e.event));

        let result = state.execute(Command::Add(-1isize as usize));
        assert!(result.is_ok());
    }
    {
        let state_store = MemoryStateStore::<_, _, _, fnv::FnvBuildHasher>::default();
        let result = state_store.put_state(&0, 0, State { value: 100 });
        assert!(result.is_ok());

        let snapshot = state_store.get_state(&0);

        assert_eq!(snapshot, Ok(Some(PersistedSnapshot { version: 0, data: State { value: 100 } })));
        let snapshot = snapshot.unwrap().unwrap();
        let snapshot_version = snapshot.version;

        let new_events = es.read(&0, Since::Offset(snapshot.version)).unwrap().unwrap();
        let new_state = new_events.iter().fold(snapshot, |s, e| PersistedSnapshot { version: e.offset, data: s.data.apply(e.event) });

        assert_eq!(new_state.data, State { value: 256 });

        let result = new_state.data.execute(Command::DivideBy(25));
        assert!(result.is_ok());

        let result_x = es.append_events(&0, &vec![(Event::Added(25), ())], Precondition::Always);
        assert!(result_x.is_ok());

        let decorated_events: Vec<_> = result.unwrap().into_iter()
            .map(|e| (e, ()))
            .collect();

        let result = es.append_events(&0, &decorated_events, Precondition::LastOffset(snapshot_version));
        assert!(result.is_err());

        let result = state_store.put_state(&0, new_state.version, new_state.data);
        assert!(result.is_ok());
    }
}

// Things to think about:
// - When writing, have preconditions to appending events
//   - Expected Last Offset: Offset
//   - EmptyStream / NoStream (for some backends these may be interpreted the same)
//   - Always