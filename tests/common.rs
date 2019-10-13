use std::{
    task::{Poll, Context},
    pin::Pin,
};
use futures::{
    Stream,
    Sink,
    channel::mpsc,
};



pub fn spammer(tag: impl Into<String>, count: usize) -> impl Stream<Item = String> + Unpin {
    Spammer::new(tag, count)
}

pub fn barrel<I>() -> Barrel<I>
    where I: Unpin
{
    Barrel::new()
}


impl<I> Barrel<I>
    where I: Unpin
{

    pub fn new() -> Self {
	let (tx, rx) = mpsc::unbounded();
	Self {
	    things: rx,
	    handle: tx,
	}
    }

    pub fn collect(&mut self) -> Vec<I> {
	let mut res = Vec::new();

	while let Ok(Some(item)) = self.things.try_next() {
	    res.push(item);
	}

	res

    }

    pub fn pipe(&self) -> Pipe<I> {
	Pipe {
	    tx: self.handle.clone(),
	}
	
    }
}


pub struct Barrel<I>
    where I: Unpin
{
    things: mpsc::UnboundedReceiver<I>,
    handle: mpsc::UnboundedSender<I>,
}



pub struct Pipe<I>
{
    tx: mpsc::UnboundedSender<I>,
}




impl<I> Sink<I> for Pipe<I>
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

    fn start_send(mut self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
	self.as_mut().tx.unbounded_send(item).unwrap();
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

	let number = std::time::SystemTime::now()
	    .duration_since(std::time::UNIX_EPOCH)
	    .map(|d| d.as_micros())
	    .unwrap_or(100);

	Poll::Ready(Some(format!("{} says `{}`", self.tag.clone(), number)))
    }
    
}
