use futures::Future;
use enande::Processor;
use enande::ProcRes;
use std::pin::Pin;

mod common;

pub struct Error {
    pub message: String,
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
	Error { message: format!("This did not happen!") }
    }
}

struct Simple;

impl enande::Processor for Simple {
    type Item = String;
    type Error = Error;
    type ResultItem = String;


    fn process(&mut self, _item: Self::Item) -> Pin<Box<dyn Future<Output = Result<ProcRes<Self::ResultItem>, Self::Error>> + Send + '_>> {
	use futures::FutureExt;
	self.actual_process().boxed()
    }

    
}

impl Simple {

    async fn actual_process(&mut self) -> Result<ProcRes<String>, Error> {
	Ok(ProcRes::One("A String".to_owned()))
    }
}

#[test]
fn test_som_spammers() {
    let mut b = Simple::process_builder();
    let mut barrel = common::barrel();

    for i in 0..100 {
	b.add_stream(common::spammer(format!("Spammer #{}", i), 100));
    }

    let fut = b.run(barrel.pipe(), Simple);
    let mut rt = tokio::runtime::current_thread::Runtime::new().expect("Creating runtime");


    rt.block_on(fut).unwrap();

    assert_eq!(100*100, barrel.collect().len());
}



