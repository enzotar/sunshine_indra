pub trait Store {
    type Error;
    fn execute(&mut self, msg: msg::Msg) -> Result<msg::Reply, Self::Error>;
}

pub mod msg;
