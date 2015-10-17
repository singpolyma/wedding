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
use urlencoded::{UrlEncodedBody, UrlEncodedQuery};
use sqlite3::{Query, ResultRowAccess, StatementUpdate};
use sqlite3::types::{ToSql};
use sqlite3::core::{ResultRow, PreparedStatement, DatabaseConnection};
use rustache::HashBuilder;

fn option_map_mut<T, F: FnOnce(T) -> T>(x: &mut Option<T>, f: F) {
	match *x {
		Some(_) => {
			*x = Some(f(x.take().unwrap()));
		}
		None => {}
	}
}

fn query_fold<T, F: Fn(&mut ResultRow, T) -> T>(z: T, q: &mut PreparedStatement, values: &[&ToSql], f: F) -> T
{
	let mut zbox = Some(z);

	q.query(values, &mut |row| {
		option_map_mut(&mut zbox, { |z| f(row, z) });
		Ok(())
	}).unwrap();

	zbox.unwrap()
}

fn render_home(db: DatabaseConnection, msg: Option<&str>) -> IronResult<Response> {
	Ok(Response::with(
		(
			iron::modifiers::Header(iron::headers::ContentType::html()),
			status::Ok,
			rustache::render_file("views/home.mustache",
				HashBuilder::new().
				insert_string("message", msg.unwrap_or("")).
				insert_vector("registry", |registry| {
					query_fold(
						registry,
						&mut db.prepare("SELECT *, want - bought AS need FROM registry WHERE (want - bought) > 0").unwrap(),
						&[],
						|row, vec| {
							let id = row.get::<&str, i32>("id");
							let title = row.get::<&str, String>("title");
							let url = row.get::<&str, String>("url");
							let note = row.get::<&str, String>("note");
							let photo = row.get::<&str, String>("photo");
							let need = row.get::<&str, i32>("need");
							let exactly = row.get::<&str, bool>("exactly");
							vec.push_hash( |hsh| {
								hsh.
									insert_int("id", id).
									insert_string("title", title.clone()).
									insert_string("url", url.clone()).
									insert_string("note", note.clone()).
									insert_string("photo", photo.clone()).
									insert_int("need", need).
									insert_bool("exactly", exactly)
							})
						}
					)
				})
			).unwrap().unwrap()
		)
	))
}

fn main() {

	fn home(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();
		return render_home(db, None);
	}

	fn registry_post(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		let body = req.get_ref::<UrlEncodedBody>().ok();

		match body.and_then( |x| x.get("id") ).and_then( |x| x.first() ) {
			Some(id) => {
				db.prepare("UPDATE registry SET bought=bought+1 WHERE id=?1").unwrap().
					update(&[id]).
					unwrap();

				render_home(db, Some("Your purchase has been recorded in our registry.  Thanks!"))
			}
			None =>
				Ok(Response::with((status::BadRequest, "Bad request.")))
		}
	}

	fn rsvp_search(req: &mut Request) -> IronResult<Response> {
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			req.get_ref::<UrlEncodedQuery>().ok().
				and_then( |x| x.get("q") ).
				and_then( |x| x.first() ).
				and_then( |x| if(x.len() > 3) { Some(x) } else { None } ).
				map_or(
					(iron::modifiers::Header(iron::headers::ContentType::plaintext()), status::BadRequest, "Search no good, go back and try again.".to_string().into_bytes()),
					|q|
						(iron::modifiers::Header(iron::headers::ContentType::html()), status::Ok, rustache::render_file("views/rsvp_results.mustache", HashBuilder::new().insert_vector("guests", |guests| {
							query_fold(
								guests,
								&mut db.prepare("SELECT * FROM guests WHERE fn LIKE ?1 OR email LIKE ?1 OR search_name LIKE ?1").unwrap(),
								&[&format!("%{}%", q)],
								|row, vec| {
									let name = row.get::<&str, String>("fn");
									let id = row.get::<&str, i32>("id");
									vec.push_hash( |guest|
										guest.
											insert_string("fn", name.clone()).
											insert_int("id", id)
									)
								}
							)
						})).unwrap().unwrap())
				)
		))
	}

	fn rsvp_form(req: &mut Request) -> IronResult<Response> {
		let guestid : i32 = req.extensions.get::<Router>().unwrap().find("guestid").unwrap().parse().unwrap();
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		Ok(Response::with(
			(
				iron::modifiers::Header(iron::headers::ContentType::html()),
				status::Ok,
				rustache::render_file("views/rsvp_form.mustache",
					query_fold(
						HashBuilder::new(),
						&mut db.prepare("SELECT * FROM guests WHERE id=$1").unwrap(),
						&[&guestid],
						|row, hsh| {
							hsh.insert_string("fn", row.get::<&str, String>("fn"))
						}
					)
				).unwrap().unwrap()
			)
		))
	}

	fn rsvp(req: &mut Request) -> IronResult<Response> {
		let guestid : i32 = req.extensions.get::<Router>().unwrap().find("guestid").unwrap().parse().unwrap();
		let db = sqlite3::access::open("db.sqlite3", None).unwrap();

		let body = req.get_ref::<UrlEncodedBody>().ok();
		let coming = body.and_then( |x| x.get("coming") ).and_then( |x| x.first() ).map( |x| x == "coming" ).unwrap_or(false);
		let vegetarian = body.and_then( |x| x.get("vegetarian") ).and_then( |x| x.first() ).and_then( |x| x.parse().ok() ).unwrap_or(0);
		let carpool = body.and_then( |x| x.get("carpool") ).and_then( |x| x.first() ).map( |x| x.clone() );
		let songs = body.and_then( |x| x.get("songs") ).and_then( |x| x.first() ).map( |x| x.clone() );
		let note = body.and_then( |x| x.get("note") ).and_then( |x| x.first() ).map( |x| x.clone() );

		db.prepare("UPDATE guests SET coming=?2, vegetarian=?3, carpool=?4, songs=?5, note=?6 WHERE id=?1").unwrap().
			update(&[&guestid, &coming, &vegetarian, &carpool, &songs, &note]).
			unwrap();

		Ok(Response::with(
			(
				iron::modifiers::Header(iron::headers::ContentType::html()),
				status::Ok,
				rustache::render_file("views/rsvp_done.mustache", HashBuilder::new()).unwrap().unwrap()
			)
		))
	}

	let mut router = Router::new();
	router.get("/", home);
	router.post("/", registry_post);
	router.get("/rsvp", rsvp_search);
	router.get("/rsvp/:guestid", rsvp_form);
	router.post("/rsvp/:guestid", rsvp);

	let mut chain = Chain::new(router);
	chain.link(Logger::new(None));

	println!("Starting on port 3000...");
	Iron::new(chain).http("localhost:3000").unwrap();
}
