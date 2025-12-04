# slintrust

`slintrust` is a lightweight, Rust-based ORM designed to simplify database operations using procedural macros. It eliminates the need for manually writing table schemas, migrations, or repetitive SQL commands.

---

## Features

- Simple table definition using `#[slint]` macro.
- Automatic schema generation and migrations.
- Supports basic CRUD operations: insert, query, get all, and raw SQL execution.
- Works asynchronously with `tokio` and `sqlx`.
- Multi-table support.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
slintrust = "0.1.0"
```

### Example usage

```rust
use slintrust::*;

use serde::{Deserialize, Serialize};

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
    // #[slint_field(primary_key)]
    pub id: String,
    pub name: String,
    pub email: String,
}

pub struct H {
    //    #[slint_field(primary_key)]
    pub id: String,
}

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    let mut orm = OrmStruct::new(
        "postgres://postgres@localhost:5432/postgres?connect_timeout=10".into(),
        vec![User::slint_schema(), Postsx::slint_schema()],
    );

    orm.connect().await?;
    orm.migrate().await?;

    let new_user = User {
        id: "".into(),
        name: "Ada".into(),
        email: "ada@mail.com".into(),
    };
    let new_post = Postsx {
        id: "".into(),
        name: "Ada".into(),
        email: "ada@mail.com".into(),
    };
    orm.insert("userx_table", &new_user).await?;
    orm.insert("postsx_table", &new_post).await?;
    let uu: Vec<User> = orm
        .query("userx_table")
        .limit(2)
        .like("name", "Ad")
        .fetch_all()
        .await?;
    println!("filtered {:?}", uu);
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

```
# slintrust
