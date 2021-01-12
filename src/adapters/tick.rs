pub trait Tick: Send {
    // const FREQ: u32;
    fn freq() -> u32;
}
