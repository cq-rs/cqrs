extern crate cqrs;
extern crate cqrs_data;
extern crate hyper;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

#[cfg(test)] #[macro_use] extern crate static_assertions;

mod http;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
