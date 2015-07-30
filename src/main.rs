extern crate iron;
extern crate logger;
extern crate router;
extern crate urlencoded;
extern crate sqlite3;
extern crate rustache;

use iron::prelude::*;
use iron::status;
use logger::Logger;
use router::Router;
use urlencoded::UrlEncodedBody;
use sqlite3::{Query, ResultRowAccess};
use rustache::HashBuilder;

fn option_map_mut<T, F: FnOnce(T) -> T>(x: &mut Option<T>, f: F) {
	match *x {
		Some(_) => {
			*x = Some(f(x.take().unwrap()));
		}
		None => {}
	}
}

fn main() {

	fn rsvp(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			req.get_ref::<UrlEncodedBody>().ok().
				and_then( |x| x.get("q") ).
				and_then( |x| x.first() ).
				map_or(
					(status::BadRequest, "Invalid POST body.\n".to_string().into_bytes()),
					|q|
						(status::Ok, rustache::render_file("views/rsvp_results.mustache", HashBuilder::new().insert_vector("guests", |guests| {
							let mut guests_box = Some(guests);
							db.prepare("SELECT * FROM guests WHERE fn LIKE $1 OR email LIKE $1").unwrap().
								query(&[&format!("%{}%", q)], &mut |row| {
									let name = row.get::<&str, String>("fn");
									option_map_mut(&mut guests_box, |vec|
										vec.push_hash( |guest|
											guest.insert_string("fn", name.clone())
										)
									);
									Ok(())
								}).unwrap();
							guests_box.unwrap()
						})).unwrap().unwrap())
				)
		))
	}

	let mut router = Router::new();
	router.post("/rsvp", rsvp);

	let mut chain = Chain::new(router);
	chain.link(Logger::new(None));

	Iron::new(chain).http("localhost:3000").unwrap();

	println!("Started...");
}
