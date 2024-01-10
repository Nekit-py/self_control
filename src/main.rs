use chrono::Local;
use rusqlite::{named_params, params, Connection, Result};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

enum QueryPath {
    Create,
    Insert,
    Read,
    Update,
    Delete,
}

impl QueryPath {
    fn as_pathbuf(&self) -> PathBuf {
        match self {
            QueryPath::Create => PathBuf::from("src/db/sql/create.sql"),
            QueryPath::Insert => PathBuf::from("src/db/sql/add.sql"),
            QueryPath::Read => PathBuf::from("src/db/sql/read.sql"),
            QueryPath::Update => PathBuf::from("src/db/sql/update.sql"),
            QueryPath::Delete => PathBuf::from("src/db/sql/delete.sql"),
        }
    }
}

#[inline]
fn query_from(path: QueryPath) -> Result<String, Box<dyn Error>> {
    let path = path.as_pathbuf();
    let query = fs::read_to_string(path)?;
    Ok(query)
}

enum Status {
    New,
    InProcess,
    Completed,
}
impl Status {
    fn to_string(&self) -> String {
        match self {
            Status::New => "Новая".to_owned(),
            Status::InProcess => "В работе".to_owned(),
            Status::Completed => "Завершена".to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub create_date: String,
    pub status: String,
    pub deleted: u8,
}

pub struct AddTask {
    pub title: String,
    pub description: String,
    create_date: String,
    status: String,
    deleted: u8,
}

impl AddTask {
    fn new(title: String, description: String) -> Self {
        AddTask {
            title,
            description,
            create_date: {
                let date = Local::now();
                date.format("%Y-%m-%d %H:%M:%S").to_string()
            },
            status: Status::New.to_string(),
            deleted: 0,
        }
    }
}

pub struct UpdateTask {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub deleted: Option<u8>,
}

impl Default for UpdateTask {
    fn default() -> Self {
        UpdateTask {
            title: Some(String::default()),
            description: Some(String::default()),
            status: Some(String::default()),
            deleted: Some(1),
        }
    }
}

fn create_table(conn: &Connection) -> Result<()> {
    // Создание таблицы
    match query_from(QueryPath::Create) {
        Ok(query) => {
            conn.execute(query.as_str(), ())?;
            println!("Таблица Tasks создана");
        }
        Err(_) => panic!("Не удалось соединиться с базой."),
    }
    Ok(())
}

fn add_task(conn: &Connection, task: AddTask) -> Result<()> {
    // Добавление заметки
    match query_from(QueryPath::Insert) {
        Ok(query) => {
            // conn.execute(query.as_str(), task.into());
            let _ = conn.execute(
                query.as_str(),
                (
                    &task.title,
                    &task.description,
                    &task.create_date,
                    &task.status,
                    &task.deleted,
                ),
            );
            println!("Задача успешно добавлена");
        }
        Err(_) => panic!("Не удалось добавить задачу."),
    }
    Ok(())
}

fn task_by_title(conn: &Connection, title: &mut String) -> Result<Vec<Option<Task>>> {
    // Поиск заметок по заголовку
    title.push('%');
    match conn.prepare("SELECT * FROM tasks where title like ?") {
        Ok(mut smtp) => {
            let tasks = smtp.query_map(params![*title], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    create_date: row.get(3)?,
                    status: row.get(4)?,
                    deleted: row.get(5)?,
                })
            })?;
            let geted_tasks: Vec<Option<Task>> = tasks.into_iter().map(|task| task.ok()).collect();

            Ok(geted_tasks)
        }
        Err(e) => Err(e),
    }
}

fn task_by_id(conn: &Connection, id: u64) -> Result<Task> {
    // Поиск заметки по id
    match conn.prepare("SELECT * FROM tasks where id = ?") {
        Ok(mut smtp) => {
            let task = smtp.query_row(params![id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    create_date: row.get(3)?,
                    status: row.get(4)?,
                    deleted: row.get(5)?,
                })
            })?;

            Ok(task)
        }
        Err(e) => Err(e),
    }
}

fn update_task(conn: &Connection, id: u64, update_task: UpdateTask) -> Result<()> {
    let stmt = conn.execute(
        "UPDATE tasks SET (title) = :title, (description) = :description, (status) = :status, (deleted) = :deleted WHERE (id) = :id",
        named_params! {
            ":id": id,
            ":title": update_task.title,
            ":description": update_task.description,
            ":status": update_task.status,
            ":deleted": update_task.deleted,
        }
    )?;
    // stmt.update_rows()
    Ok(())
}

fn main() {
    let conn = match Connection::open("tasks.db") {
        Ok(conn) => conn,
        Err(_) => panic!("Не удалось подключиться к базе данных..."),
    };

    // let _ = create_table(&conn);

    // let new_task = AddTask::new("Вторая таска".to_string(), "Слово чушпана".to_string());
    // let _ = add_task(&conn, new_task);

    let finded_task = task_by_id(&conn, 1).ok();
    println!("{:?}", finded_task);

    let _ = update_task(&conn, 1, UpdateTask::default()).ok();
    println!("{:?}", finded_task);

    let finded_tasks = task_by_title(&conn, &mut "%таск".to_owned()).ok();
    println!("{:?}", finded_tasks);
}
