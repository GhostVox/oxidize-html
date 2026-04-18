use html5ever::driver::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::RcDom;

pub fn parse(html: &str) -> RcDom {
    parse_document(RcDom::default(), Default::default()).one(html)
}
