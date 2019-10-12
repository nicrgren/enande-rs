use futures::Future;
use middleman::Builder;

mod common;

struct Error {
    message: String,
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
	Error { message: format!("This did not happen!") }
    }
}

struct Processor;

impl middleman::Processor for Processor {
    type Item = String;
    type Error = Error;
    type ResultItem = String;
    type ResultFuture = Box<dyn Future<Output = Result<Self::ResultItem, Self::Error>> + Unpin>;


    fn process(&mut self, item: Self::Item) -> Self::ResultFuture {
	println!("Processing: {}", item);
	let res = futures::future::ready(Ok("A String".to_owned()));
	Box::new(res)
    }

    
}

#[test]
fn fail() {
    let mut b = Builder::<String, Error>::new();
    let barrel = common::barrel();
    let fut = 
	b
	.add_stream(common::spammer("Spammer #1", 100))
	.add_stream(common::spammer("Spammer #2", 100))
	.add_stream(common::spammer("Spammer #3", 100))
	.add_stream(common::spammer("Spammer #4", 100))
	.add_stream(common::spammer("Spammer #5", 100))
	.run(barrel, Processor);

    let mut rt = tokio::runtime::current_thread::Runtime::new().expect("Creating runtime");

    rt.block_on(fut);

}



