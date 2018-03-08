use std::fmt;

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventNumber(usize);

impl EventNumber {
    #[inline]
    pub fn new(number: usize) -> Self {
        EventNumber(number)
    }

    #[inline]
    pub fn number(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn incr(&self) -> Self {
        EventNumber(self.0 + 1)
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
pub struct VersionedSnapshot<Snapshot> {
    pub version: Version,
    pub snapshot: Snapshot,
}
