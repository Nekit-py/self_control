#![allow(dead_code)]
use chrono::Local;
use crossterm::{
    event::{self, Event::Key, KeyCode::Char},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::{CrosstermBackend, Frame, Terminal},
    widgets::Paragraph,
};
use rusqlite::{named_params, params, Connection, Result};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf; //::{stdout, Result};

enum QueryPath {
    Create,
    Insert,
}

impl QueryPath {
    fn as_pathbuf(&self) -> PathBuf {
        match self {
            QueryPath::Create => PathBuf::from("src/db/sql/create.sql"),
            QueryPath::Insert => PathBuf::from("src/db/sql/add.sql"),
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

impl Task {
    fn from(self, update_task: UpdateTask) -> Self {
        Task {
            id: self.id,
            title: update_task.title.unwrap_or(self.title),
            description: update_task.description.unwrap_or(self.description),
            create_date: self.create_date,
            status: update_task.status.unwrap_or(self.status),
            deleted: update_task.deleted.unwrap_or(self.deleted),
        }
    }
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

fn delete_task(conn: &Connection, id: u64) -> Result<()> {
    let stmt = conn.execute(
        "UPDATE tasks SET deleted = 1 WHERE id = :id",
        named_params! {
            ":id": id,
        },
    )?;
    Ok(())
}

fn update_task(conn: &Connection, id: u64, update_task: UpdateTask) -> Result<()> {
    let mut current_task = task_by_id(conn, id).ok();
    match current_task {
        Some(ct) => {
            let _stmt = conn.execute(
                "UPDATE tasks SET (title) = :title, (description) = :description, (status) = :status, (deleted) = :deleted WHERE (id) = :id",
                named_params! {
                    ":id": id,
                    ":title": update_task.title,
                    ":description": update_task.description,
                    ":status": update_task.status,
                    ":deleted": update_task.deleted,
                }
            )?;
        }
        None => {}
    }
    // stmt.update_rows()
    Ok(())
}

fn between_dates(
    conn: &Connection,
    start_date: String,
    end_date: String,
) -> Result<Vec<Option<Task>>> {
    let mut stmt = conn.prepare("SELECT * FROM tasks WHERE create_date BETWEEN ?1 AND ?2")?;

    let tasks = stmt.query_map(params![start_date, end_date], |row| {
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

fn test_db_func() {
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

    let _ = delete_task(&conn, 2);

    let finded_tasks = task_by_title(&conn, &mut "".to_owned()).ok();
    println!("{:?}", finded_tasks);

    let between = between_dates(&conn, "2024-01-09".to_string(), "2024-01-11".to_string()).ok();
    println!("{:?}", between);
}

struct App {
    counter: i64,
    should_quit: bool,
}

fn startup() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stderr(), EnterAlternateScreen)?;
    Ok(())
}
fn shutdown() -> io::Result<()> {
    execute!(std::io::stderr(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn tui(app: &App, f: &mut Frame) {
    f.render_widget(
        Paragraph::new(format!("Counter: {}", app.counter)),
        f.size(),
    );
}

fn update(app: &mut App) -> io::Result<()> {
    if event::poll(std::time::Duration::from_millis(250))? {
        if let Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('j') => app.counter += 1,
                    Char('k') => app.counter -= 1,
                    Char('q') => app.should_quit = true,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn run() -> io::Result<()> {
    // ratatui terminal
    let mut t = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    // application state
    let mut app = App { counter: 0, should_quit: false };

    loop {
        // application render
        t.draw(|f| {
            tui(&app, f);
        })?;

        // application update
        update(&mut app)?;

        // application exit
        if app.should_quit {
        break;
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    startup()?;
    let status = run();
    shutdown()?;
    status?;
    // io::stdout().execute(EnterAlternateScreen)?;
    // enable_raw_mode()?;
    // let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    // terminal.clear()?;

    // // TODO main loop
    // loop {
    //     terminal.draw(|frame| {
    //         let area = frame.size();
    //         frame.render_widget(
    //             Paragraph::new("Hello Ratatui! (press 'q' to quit)")
    //                 .white()
    //                 .on_blue(),
    //             area,
    //         );
    //     })?;

    //     if event::poll(std::time::Duration::from_millis(16))? {
    //         if let event::Event::Key(key) = event::read()? {
    //             if key.kind == KeyEventKind::Press
    //                 && key.code == KeyCode::Char('q')
    //             {
    //                 break;
    //             }
    //         }
    //     }
    // }

    // io::stdout().execute(LeaveAlternateScreen)?;
    // disable_raw_mode()?;
    // Ok(())
    Ok(())
}
