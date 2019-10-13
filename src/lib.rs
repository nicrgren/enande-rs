
//! Tries to define and specify the behaviour used in CPC and other solutions
//! where you have one ore more streams delivering data
//! and then 0 or 1 sink that sends data out.

use futures::{Sink, stream::Stream, StreamExt, Future, SinkExt};
use std::convert::TryFrom;
use std::pin::Pin;

pub enum ProcRes<T> {
    None,
    One(T),
    Many(Vec<T>),
}

impl<T> From<Option<T>> for ProcRes<T> {

    fn from(opt: Option<T>) -> Self {
	match opt {
	    Some(v) => Self::One(v),
	    None => Self::None,
	}
    }
}


pub trait Processor
where Self::Error: Send + 'static,
      Self::Item: Send + 'static,
      Self: Send + Unpin + Sized,
      Self::ResultItem: Send,
{
    type Item;
    type Error;
    type ResultItem;

    fn process(
	&mut self,
	item: Self::Item
    ) -> Pin<Box<dyn Future<Output = Result<ProcRes<Self::ResultItem>, Self::Error>> + Send + '_>>;


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
      TError: From<TSinkError> + Send + 'static,
      TItem: Send + 'static,
      TSinkItem: Send,

{

    fn new(stream: TStream, sink: TSink) -> Self {
	Self {
	    stream,
	    sink,
	    _marker: Default::default(),
	}
    }

    async fn run<P>(mut self, mut processor: P) -> Result<(), ()>
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
		    return Ok(());
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
    streams: Vec<Box<dyn Stream<Item = Result<TItem, TError>> + Send + Unpin>>
}

impl
    <TItem,
     TError>

    ProcessBuilder
    <TItem,
     TError>

where
    TError: Sized + Send + 'static,
    TItem: Sized + Send + 'static,
{

    pub fn new() -> Self {
	Self { streams: Vec::new() }
    }


    /// Adds another stream as an input source to this builder.
    pub fn add_stream<'a, TStream, TStreamItem>(&'a mut self, stream: TStream) -> &mut Self
    where
	 TStream: Stream<Item = TStreamItem> + Send + Unpin + 'static,
	 TStreamItem: Into<TItem>,
    {
	self.streams.push(Box::new(stream.map(|item| Ok(item.into()))));
	self
    }

    /// Adds an input where the Item can be converted into our Item using `TryFrom`.
    pub fn add_try<'a, TStream, TStreamItem>(&'a mut self, stream: TStream) -> &mut Self
    where
	 TStream: Stream<Item = TStreamItem> + Send + Unpin + 'static,
	 TItem: TryFrom<TStreamItem, Error = TError>,
    {

	let stream = stream.map(|stream_item| TItem::try_from(stream_item));
	self.streams.push(Box::new(stream));
	self
    }


    /// Adds an input that contains fails.
    pub fn add_try_stream<'a, TStream, TStreamError>(&'a mut self, stream: TStream) -> &mut Self
    where
	TStream: Stream<Item = Result<TItem, TStreamError>> + Send + Unpin + 'static,
	TStreamError: 'static,
	TError: From<TStreamError>,
    {
	use futures::TryStreamExt;

	let stream = stream
	    .map_err(TError::from);

	self.streams.push(Box::new(stream));
	self
    }

    pub fn run<TSink, TSinkError, TProcessor>(
	self,
	sink: TSink,
	processor: TProcessor
    ) -> impl Future<Output = Result<(), ()>> + Send
    where
	TProcessor: Processor<Item = TItem, Error = TError> + 'static,
	TSink: Sink<TProcessor::ResultItem, Error = TSinkError> + Send + Unpin + 'static,
	TSinkError: 'static,
	TError: From<TSinkError> + Send + 'static
    {
	use futures::FutureExt;
	self.run_impl(sink, processor).boxed()
    }



    pub async fn run_impl<TSink, TSinkError, TProcessor>(
	mut self,
	sink: TSink,
	processor: TProcessor
    ) -> Result<(), ()>
	where
	TProcessor: Processor<Item = TItem, Error = TError>,
	TSink: Sink<TProcessor::ResultItem, Error = TSinkError> + Unpin,
	TError: From<TSinkError> + Send + 'static

    {
	let joined_streams = futures::stream::select_all(self.streams.drain(0..));
	let runner = Runner::new(joined_streams, sink);

	runner.run(processor).await
	
    }

}
