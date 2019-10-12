use std::{
    task::{Poll, Context},
    pin::Pin,
};
use futures::{Stream, Sink};



pub fn spammer(tag: impl Into<String>, count: usize) -> impl Stream<Item = String> + Unpin {
    Spammer::new(tag, count)
}

pub fn barrel<I>() -> Barrel<I>
    where I: Unpin
{
    Barrel { things: Vec::new() }
}

impl<I> Barrel<I>
    where I: Unpin
{
    pub fn collected(self) -> Vec<I> {
	self.things
    }
}



pub struct Barrel<I>
    where I: Unpin
{
    things: Vec<I>,
} 

impl<I> Sink<I> for Barrel<I>
    where I: Unpin
{

    type Error = std::convert::Infallible;

    fn poll_ready(
	self: Pin<&mut Self>,
	_: &mut Context
    ) -> Poll<Result<(), Self::Error>>
    {
	Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
	self.get_mut().things.push(item);
	Ok(())
    }

    fn poll_flush(
	self: Pin<&mut Self>,
	_: &mut Context
    ) -> Poll<Result<(), Self::Error>>
    {
	Poll::Ready(Ok(()))
    }

    fn poll_close(
	self: Pin<&mut Self>,
	_: &mut Context
    ) -> Poll<Result<(), Self::Error>>
    {
	
	Poll::Ready(Ok(()))
    }

}



pub struct Spammer {
    tag: String,
    count: usize,
}

impl Spammer {
    fn new(tag: impl Into<String>, count: usize) -> Self {
	Self {
	    tag: tag.into(),
	    count,
	}
    }
}

impl Stream for Spammer {

    type Item = String;

    fn poll_next(
	mut self: Pin<&mut Self>,
	_: &mut Context,
    ) -> Poll<Option<Self::Item>> {

	if self.count == 0 {
	    return Poll::Ready(None);
	}

	self.count -= 1;


	println!("Spammer polled");
	let number = std::time::SystemTime::now()
	    .duration_since(std::time::UNIX_EPOCH)
	    .map(|d| d.as_micros())
	    .unwrap_or(100);

	Poll::Ready(Some(format!("{} says `{}`", self.tag.clone(), number)))
    }
    
}
