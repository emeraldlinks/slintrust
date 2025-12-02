
use slintrust::*;

use serde::{Serialize, Deserialize};



#[slint(table_name="userx_table")]
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}



#[slint(table_name="postsx_table")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Postsx {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[tokio::main] 
async fn main() -> sqlx::Result<()> {
    let mut orm = OrmStruct::new(
        "postgres://postgres@localhost:5432/postgres?connect_timeout=10".into(),
        vec![User::slint_schema(), Postsx::slint_schema()]
    );

    orm.connect().await?;
    orm.migrate().await?;

    let new_user = User { id: "".into(), name: "Ada".into(), email: "ada@mail.com".into() };
    let new_post = Postsx { id: "".into(), name: "Ada".into(), email: "ada@mail.com".into() };
    orm.insert("userx_table", &new_user).await?;
    orm.insert("postsx_table", &new_post).await?;

    let ada: Option<User> = orm.first("userx_table", "email", "ada@mail.com").await?;
    let posts: Option<Postsx> = orm.first("userx_table", "email", "ada@mail.com").await?;
    println!("{:?}", ada);
    println!("posts {:?}", posts);

    let all_users: Vec<User> = orm.get_all("userx_table").await?;
    let all_posts: Vec<Postsx> = orm.get_all("postsx_table").await?;
    println!("All users: {:?}", all_users);
    println!("All posts: {:?}", all_posts);

    orm.raw("DELETE FROM \"user\" WHERE id='1'").await?;

    Ok(())
}
