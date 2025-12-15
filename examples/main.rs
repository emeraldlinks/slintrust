use slint_derive::slint;
use slintrust::*;

use serde::{Deserialize, Serialize};
use serde_json::json;

// adjust path if needed

// =======================
// Models
// =======================

#[slint(table_name = "userx_table")]
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[slint(table_name = "postsx_table")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Postsx {
    pub id: String,
    pub name: String,
    pub email: String,
    pub user_d: String,
}

// =======================
// Main
// =======================

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    // -----------------------
    // ORM bootstrap
    // -----------------------
    let mut orm = OrmStruct::new(
        "postgres://postgres@localhost:5432/postgres".into(),
        vec![User::slint_schema(), Postsx::slint_schema()],
    );

    orm.connect().await?;
    orm.migrate().await?;

    // =======================
    // OLD ORM API (direct)
    // =======================

    let user = User {
        id: "".into(),
        name: "Ada".into(),
        email: "ada@mail.com".into(),
    };

    orm.insert("userx_table", &user).await?;

    let users: Vec<User> = orm
        .query("userx_table")
        .like("name", "Ad")
        .limit(5)
        .fetch_all()
        .await?;

    println!("Users via old query API: {:?}", users.len());

    let ada: Option<User> = orm.first("userx_table", "email", "ada@mail.com").await?;

    println!("First user via old API: {:?}", ada);

    // =======================
    // NEW ORM API (typed)
    // =======================

    let user_table = Table::<User>::new(&orm, "userx_table", "email");
    let post_table = Table::<Postsx>::new(&orm, "postsx_table", "id");

    // -----------------------
    // Insert
    // -----------------------
    user_table
        .insert(&User {
            id: "".into(),
            name: "Grace".into(),
            email: "grace@mail.com".into(),
        })
        .await?;

    // -----------------------
    // Get single record
    // -----------------------
    let record = user_table.get(json!({ "email": "ada@mail.com" })).await?;

    if let Some(user) = &record {
        println!("Fetched via Table.get(): {:?}", user.value);
    }

    // -----------------------
    // Update record
    // -----------------------
    // let updated_user = user_table
    //     .update(json!({"email": "ada@mail.com"}), json!({"name": "Ada Lovelace"}))
    //     .await?;
    // println!("Updated user: {:?}", updated_user);

    // -----------------------
    // Delete record
    // -----------------------
    // user_table
    //     .delete(json!({"email": "ada@mail.com"}))
    //     .await?;
    // println!("User deleted");

    // =======================
    // ADVANCED QUERY API
    // =======================

    let queried = user_table
        .query()
        .where_clause("name", "LIKE", "%Ada%")
        .order_by("name", "ASC")
        .limit(10)
        .get()
        .await?;

    println!("Advanced query results: {}", queried.len());

    // -----------------------
    // First using query
    // -----------------------
    let first_user = user_table
        .query()
        .where_clause("email", "=", "ada@mail.com")
        .first()
        .await?;

    if let Some(userx) = first_user {
        println!("First via new Query API: {:?}", userx.value);

        let updated = userx.update(json!({"name": "joy"})).await?;
        println!("Updated user: {:?}", updated);
        // ----------------<|fim_middle|><|fim_middle|><|fim_middle|>
        // Delete
        // -----------------------
        // userx.delete().await?;
        // println!("User deleted");
    }

    let first_userx = user_table
        .query()
        .where_clause("email", "=", "ada@mail.com")
        .first_value()
        .await?;
    println!("First_Value via new Query API: {:?}", first_userx);

    Ok(())
}
