use std::fmt;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventNumber(NonZeroUsize);

impl EventNumber {
    pub const MIN_VALUE: EventNumber = EventNumber(unsafe {NonZeroUsize::new_unchecked(1)});

    #[inline]
    pub fn new(x: usize) -> Option<Self> {
        Some(EventNumber(NonZeroUsize::new(x)?))
    }

    #[inline]
    pub fn get(self) -> usize {
        self.0.get()
    }

    #[inline]
    #[must_use]
    pub fn incr(self) -> Self {
        let x = self.0.get();
        EventNumber(NonZeroUsize::new(x + 1).unwrap())
    }
}

impl fmt::Display for EventNumber {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

//impl PartialEq<usize> for EventNumber {
//    #[inline]
//    fn eq(&self, rhs: &usize) -> bool {
//        *rhs == self.0
//    }
//}
//
//impl PartialEq<EventNumber> for usize {
//    #[inline]
//    fn eq(&self, rhs: &EventNumber) -> bool {
//        rhs.0 == *self
//    }
//}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    Initial,
    Number(EventNumber),
}

impl Version {
    #[inline]
    pub fn new(number: usize) -> Self {
        NonZeroUsize::new(number)
            .map(EventNumber)
            .map(Version::Number)
            .unwrap_or(Version::Initial)
    }

    #[inline]
    #[must_use]
    pub fn incr(self) -> Self {
        match self {
            Version::Initial => Version::Number(EventNumber(NonZeroUsize::new(1).unwrap())),
            Version::Number(en) => Version::Number(en.incr()),
        }
    }

    #[inline]
    pub fn get(self) -> usize {
        match self {
            Version::Initial => 0,
            Version::Number(en) => en.get(),
        }
    }

    #[inline]
    pub fn event_number(self) -> Option<EventNumber> {
        match self {
            Version::Initial => None,
            Version::Number(en) => Some(en),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Version::Initial => f.write_str("initial"),
            Version::Number(ref event_number) => event_number.fmt(f),
        }
    }
}

impl Default for Version {
    #[inline]
    fn default() -> Self {
        Version::Initial
    }
}

//impl PartialEq<EventNumber> for Version {
//    fn eq(&self, rhs: &EventNumber) -> bool {
//        if let Version::Number(ref v) = *self {
//            v == rhs
//        } else {
//            false
//        }
//    }
//}

impl From<EventNumber> for Version {
    #[inline]
    fn from(event_number: EventNumber) -> Self {
        Version::Number(event_number)
    }
}

impl ::std::ops::Sub for Version {
    type Output = isize;

    fn sub(self, rhs: Version) -> Self::Output {
        let l = match self {
            Version::Initial => 0,
            Version::Number(n) => n.get() as isize
        };
        let r = match rhs {
            Version::Initial => 0,
            Version::Number(n) => n.get() as isize
        };

        l - r
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Precondition {
    New,
    Exists,
    ExpectedVersion(Version),
}

impl Precondition {
    pub fn verify(&self, version_opt: Option<Version>) -> Result<(), Self> {
        match *self {
            Precondition::Exists if version_opt.is_some() => Ok(()),
            Precondition::New if version_opt.is_none() => Ok(()),
            Precondition::ExpectedVersion(expected_version) if version_opt.is_some() => {
                if let Some(version) = version_opt {
                   if version == expected_version {
                       Ok(())
                   } else {
                       Err(*self)
                   }
                } else {
                    unreachable!()
                }
            }
            _ => Err(*self)
        }
    }
}

impl From<Version> for Precondition {
    #[inline]
    fn from(v: Version) -> Self {
        Precondition::ExpectedVersion(v)
    }
}

impl fmt::Display for Precondition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Precondition::Exists => f.write_str("expect aggregate exists"),
            Precondition::New => f.write_str("expect aggregate does not exist"),
            Precondition::ExpectedVersion(Version::Initial) => f.write_str("expect aggregate to exist in initial state"),
            Precondition::ExpectedVersion(Version::Number(v)) => write!(f, "expect aggregate to exist with version {}", v),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct SequencedEvent<Event>
{
    pub sequence_number: EventNumber,
    pub event: Event,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct StateSnapshot<State> {
    pub version: Version,
    pub snapshot: State,
}
