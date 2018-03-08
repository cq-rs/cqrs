pub trait Projection {
    type Event;

    fn apply(&mut self, event: Self::Event);
}

#[cfg(test)]
assert_obj_safe!(proj; Projection<Event=()>);

