
//! Tries to define and specify the behaviour used in CPC and other solutions
//! where you have one ore more streams delivering data
//! and then 0 or 1 sink that sends data out.

use futures::{Sink, stream::Stream, StreamExt, Future, SinkExt};
use std::convert::TryFrom;
use std::pin::Pin;

pub enum ProcRes<I> {
    None,
    One(I),
    Many(Vec<I>),
}


pub trait Processor
where Self::Error: 'static,
      Self::Item: 'static,
      Self: Unpin + Sized,
{
    type Item;
    type Error;
    type ResultItem;

    fn process(&mut self, item: Self::Item) -> Pin<Box<dyn Future<Output = Result<ProcRes<Self::ResultItem>, Self::Error>> + '_>>;


    fn stopped(&mut self) {}
    fn stopped_with_error(_error: Self::Error) {}
    fn on_error(&mut self, _error: Self::Error) {}


    fn process_builder() -> ProcessBuilder<Self::Item, Self::Error> {
	ProcessBuilder::new()
    }

}


pub struct Handle;


struct Runner<TStream, TItem, TError, TSink, TSinkItem, TSinkError>
where TStream: Stream<Item = Result<TItem, TError>> + Unpin,
      TSink: Sink<TSinkItem, Error = TSinkError> + Unpin,
{
    stream: TStream,
    sink: TSink,
    _marker: std::marker::PhantomData<TSinkItem>,
}


impl
    <TStream,
     TItem,
     TError,
     TSink,
     TSinkItem,
     TSinkError>
    
    Runner
    <TStream,
     TItem,
     TError,
     TSink,
     TSinkItem,
     TSinkError>

where TStream: Stream<Item = Result<TItem, TError>> + Unpin,
      TSink: Sink<TSinkItem, Error = TSinkError> + Unpin,
      TError: From<TSinkError> + 'static,
      TItem: 'static,
{

    fn new(stream: TStream, sink: TSink) -> Self {
	Self {
	    stream,
	    sink,
	    _marker: Default::default(),
	}
    }

    async fn run<P>(mut self, mut processor: P)
    where P: Processor<Item = TItem, Error = TError, ResultItem = TSinkItem>,
    {


	loop {
	    let incoming = match self.stream.next().await {
		Some(Ok(incoming)) => incoming,
		Some(Err(err)) => {
		    // Add ability for Processor to STOP here by returning a value.
		    processor.on_error(err);
		    continue;
		},

		None => {

		    println!("Stream empty, stopping");
		    processor.stopped();
		    return;
		}
		    
	    };

	    let proc_res = match processor.process(incoming).await {
		Ok(proc_res) => proc_res,
		Err(err) => {
		    processor.on_error(err);
		    continue;
		}
	    };


	    match proc_res {
		ProcRes::None => continue,


		ProcRes::One(item) => {
		    if let Err(_failed) = self.sink.send(item).await {
			// @TODO: Maybe deliver this failed thing to the
			// processor. Allow it to act on the failed item.
			// for now, we just drop it and continue.
			continue;
		    }

		    if let Err(err) = self.sink.flush().await {
			processor.on_error(TError::from(err));
		    }
		}


		ProcRes::Many(v) => {
		    let mut st = futures::stream::iter(v.into_iter());
		    if let Err(err) = self.sink.send_all(&mut st).await {
			processor.on_error(TError::from(err))
		    }
		    
		}
		
	    }
	}
    }
    

    
}

pub struct ProcessBuilder<TItem, TError>{
    streams: Vec<Box<dyn Stream<Item = Result<TItem, TError>> + Unpin>>
}

impl
    <TItem,
     TError>

    ProcessBuilder
    <TItem,
     TError>

where
    TError: Sized + 'static,
    TItem: Sized + 'static,
{

    pub fn new() -> Self {
	Self { streams: Vec::new() }
    }


    /// Adds another stream as an input source to this builder.
    pub fn add_stream<'a, TStream, TStreamItem>(&'a mut self, stream: TStream) -> &mut Self
    where
	 TStream: Stream<Item = TStreamItem> + Unpin + 'static,
	 TStreamItem: Into<TItem>,
    {
	self.streams.push(Box::new(stream.map(|item| Ok(item.into()))));
	self
    }

    /// Adds an input where the Item can be converted into our Item using `TryFrom`.
    pub fn add_try<'a, TStream, TStreamItem>(&'a mut self, stream: TStream) -> &mut Self
    where
	 TStream: Stream<Item = TStreamItem> + Unpin + 'static,
	 TItem: TryFrom<TStreamItem, Error = TError>,
    {

	let stream = stream.map(|stream_item| TItem::try_from(stream_item));
	self.streams.push(Box::new(stream));
	self
    }


    /// Adds an input that contains fails.
    pub fn add_try_stream<'a, TStream, TStreamItem, TStreamError>(&'a mut self, stream: TStream) -> &mut Self
    where
	TStream: Stream<Item = Result<TStreamItem, TStreamError>> + Unpin + 'static,
	TStreamError: 'static,
	TStreamItem: 'static,
	TItem: TryFrom<TStreamItem, Error = TError>,
	TError: From<TStreamError>,
    {
	use futures::TryStreamExt;

	let stream = stream
	    .map_err(TError::from)
	    .and_then(|stream_item| futures::future::ready(TItem::try_from(stream_item)));

	self.streams.push(Box::new(stream));
	self
    }

    pub async fn run<TSink, TSinkError, TProcessor>(
	&mut self,
	sink: TSink,
	processor: TProcessor
    )
	where
	TProcessor: Processor<Item = TItem, Error = TError>,
	TSink: Sink<TProcessor::ResultItem, Error = TSinkError> + Unpin,
	TError: From<TSinkError>

    {
	let joined_streams = futures::stream::select_all(self.streams.drain(0..));
	let runner = Runner::new(joined_streams, sink);

	runner.run(processor).await
	
    }

}
