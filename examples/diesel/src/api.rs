use diesel::prelude::*;
use failure::Fallible;
use serde::Deserialize;

use finchers::input::query::{FromQuery, QueryItems, Serde};

use crate::database::Connection;
use crate::model::{NewPost, Post};
use crate::schema::posts;

#[derive(Debug, Deserialize)]
pub struct Query {
    count: i64,
}

impl Default for Query {
    fn default() -> Query {
        Query { count: 20 }
    }
}

impl FromQuery for Query {
    type Error = <Serde<Query> as FromQuery>::Error;

    fn from_query(items: QueryItems) -> Result<Self, Self::Error> {
        FromQuery::from_query(items).map(Serde::into_inner)
    }
}

pub async fn get_posts(query: Option<Query>, conn: Connection) -> Fallible<Vec<Post>> {
    let query = query.unwrap_or_default();
    let posts = await!(conn.execute(move |conn| {
        use crate::schema::posts::dsl::*;
        posts.limit(query.count).load::<Post>(conn.get())
    }))?;
    Ok(posts)
}

pub async fn create_post(new_post: NewPost, conn: Connection) -> Fallible<Post> {
    let post = await!(conn.execute(move |conn| {
        diesel::insert_into(posts::table)
            .values(&new_post)
            .get_result::<Post>(conn.get())
    }))?;
    Ok(post)
}

pub async fn find_post(i: i32, conn: Connection) -> Fallible<Option<Post>> {
    let post_opt = await!(conn.execute(move |conn| {
        use crate::schema::posts::dsl::{id, posts};
        posts
            .filter(id.eq(i))
            .get_result::<Post>(conn.get())
            .optional()
    }))?;
    Ok(post_opt)
}
