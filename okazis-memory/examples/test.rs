extern crate okazis;
extern crate okazis_memory;

use okazis::*;
use okazis::ReadOffset::*;
use okazis_memory::{MemoryEventStore, MemoryEventStream, MemoryStateStore};

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

    let es = okazis_memory::MemoryEventStore::default();
    {
        let mut s0 = es.open_stream(0);
        s0.append_events(vec![
            Event::Added(100),
            Event::Subtracted(36),
        ]);
    }
    {
        let mut s0 = es.open_stream(0);
        let past_events = s0.read(BeginningOfStream).unwrap();
        let state = past_events.iter().fold(State::default(), |s, e| s.apply(e.payload));

        let result = state.execute(Command::Multiply(-1isize as usize));
        assert_eq!(result, Err(CommandError::Overflow));

        let result = state.execute(Command::DivideBy(0));
        assert_eq!(result, Err(CommandError::DivideByZero));

        let result = state.execute(Command::Multiply(4));
        assert_eq!(result, Ok(vec![Event::Multiplied(4)]));

        s0.append_events(result.unwrap());
    }
    {
        let s0 = es.open_stream(0);
        let new_events = s0.read(Offset(0)).unwrap();
        let state = new_events.iter().fold(State { value: 36 }, |s, e| s.apply(e.payload));

        let result = state.execute(Command::Add(-1isize as usize));
        assert!(result.is_ok());
    }
    {
        let state_store = MemoryStateStore::<_, _, _>::default();
        state_store.put_state(0, 0, State { value: 100 });

        let snapshot = state_store.get_state(0);

        assert_eq!(snapshot, Ok(Some(PersistedState { offset: 0, state: State { value: 100 } })));
        let snapshot = snapshot.unwrap().unwrap();

        let s0 = es.open_stream(0);
        let new_events = s0.read(Offset(snapshot.offset)).unwrap();
        let new_state = new_events.iter().fold(snapshot, |s, e| PersistedState { offset: e.offset, state: s.state.apply(e.payload) });

        assert_eq!(new_state.state, State { value: 256 });

        let result = new_state.state.execute(Command::DivideBy(25));
        assert!(result.is_ok());

        s0.append_events(vec![Event::Added(25)]);

        s0.append_events(/* Precondition::LastOffset(new_state.offset), */result.unwrap());

        state_store.put_state(0, new_state.offset, new_state.state);
    }
}

// Things to think about:
// - When writing, have preconditions to appending events
//   - Expected Last Offset: Offset
//   - EmptyStream / NoStream (for some backends these may be interpreted the same)
//   - Always