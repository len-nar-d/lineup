use rusqlite::Connection;
use colored::Colorize;
use chrono::{Utc, Datelike};
use std::vec::Vec;
use clap::{Args, Parser, Subcommand};
use pad::{PadStr, Alignment};


#[derive(Copy, Clone)]
struct Month {
    id: usize,
    month: u32,
    year: i32,
}

struct Entrys {
    id: usize,
    name: String,
    amount: isize,
    is_expense: u8,
    month: Month,
}

struct Statics {
    id: usize,
    name: String,
    amount: isize,
    is_expense: u8,
}


struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Self {
        let connection = Connection::open("calendar.db").expect("couldn't create database");

        connection.execute(
            "CREATE TABLE IF NOT EXISTS month (
                id INTEGER PRIMARY KEY,
                month INTEGER NOT NULL,
                year INTEGER NOT NULL
            )",
            (),
        ).expect("couldn't create table: month");

        connection.execute(
            "CREATE TABLE IF NOT EXISTS entrys (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                amount INTEGER NOT NULL,
                is_expense INTEGER NOT NULL,
                month_id INTEGER NOT NULL REFERENCES month(id)
            )",
            (),
        ).expect("couldn't create table: entrys");

        connection.execute(
            "CREATE TABLE IF NOT EXISTS statics (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                amount INTEGER NOT NULL,
                is_expense INTEGER NOT NULL
            )",
            (),
        ).expect("couldn't create table: expenses");

        return Self {
            conn: connection,
        };
    }

    pub fn new_entry(&self, name: &str, amount: isize, month: u32, year: i32) {
        let month = self.create_month(month, year);

        let _ = self.conn.execute(
            "INSERT INTO entrys (name, amount, is_expense, month_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, amount, self.is_expense(amount), month.id],
        ).expect("couldn't insert into: entrys");
    }

    pub fn get_entrys(&self, month: u32, year: i32) -> Vec<Entrys> {
        let stmt = &mut self.conn.prepare(
            "
            SELECT entrys.id, entrys.name, entrys.amount, entrys.is_expense,
                   month.id, month.month, month.year
            FROM entrys, month
            WHERE entrys.month_id = month.id AND month.month = ?1
            "
        ).expect("couldn't prepare statement");

        let entrys_iter = stmt.query_map([month], |row| {
            Ok(
                Entrys {
                    id: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    amount: row.get(2).unwrap(),
                    is_expense: row.get(3).unwrap(),
                    month: Month {
                        id: row.get(4).unwrap(),
                        month: row.get(5).unwrap(),
                        year: row.get(6).unwrap(),
                    },
                } 
            )
        }).expect("couldn't create 'month_iter'");

        return entrys_iter.map(|stat| stat.unwrap()).collect();
    }

    pub fn new_static(&self, name: &str, amount: isize) {
        let _ = self.conn.execute(
            "INSERT INTO statics (name, amount, is_expense) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, amount, self.is_expense(amount)],
        ).expect("couldn't insert into: statics");
    }

    pub fn delete_static(&self, id: usize) {
        let _ = self.conn.execute(
            "DELETE FROM statics WHERE id=?1",
            rusqlite::params![id],
        ).expect("couldn't delete from: statics");
    }
    
    pub fn get_statics(&self) -> Vec<Statics> {
        let stmt = &mut self.conn.prepare(
            "SELECT id, name, amount, is_expense FROM statics"
        ).expect("couldn't prepare statement");

        let month_iter = stmt.query_map([], |row| {
            Ok(
                Statics {
                    id: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    amount: row.get(2).unwrap(),
                    is_expense: row.get(3).unwrap(),
                } 
            )
        }).expect("couldn't create 'month_iter'");

        return month_iter.map(|stat| stat.unwrap()).collect();
    }

    fn create_month(&self, month: u32, year: i32) -> Month {
        let stmt_check = &mut self.conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM month WHERE month=?1)"
        ).expect("couldn't prepare statement");

        let check: Vec<usize> = stmt_check.query_map([month], |row| { Ok(row.get(0).unwrap(),) })
                                .expect("couldn't read: month").map(|mon| mon.unwrap())
                                .collect();

        if check[0] == 0 {
            let _ = self.conn.execute(
                "INSERT INTO month (month, year) VALUES (?1, ?2)",
                rusqlite::params![month, year],
            ).expect("couldn't insert into: month");

            let id = self.conn.last_insert_rowid() as usize;

            for stat in self.get_statics() {
                let _ = self.conn.execute(
                    "INSERT INTO entrys (name, amount, is_expense, month_id) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![stat.name, stat.amount, stat.is_expense, id],
                ).expect("couldn't insert into: entrys");
            }

            return Month{
                id: id,
                month: month,
                year: year,
            };
        }

        let stmt = &mut self.conn.prepare(
            "SELECT id, month, year FROM month WHERE month=?1"
        ).expect("could't prepare statement");

        let month_iter = stmt.query_map([month], |row| {
            Ok(
                Month {
                    id: row.get(0).unwrap(),
                    month: row.get(1).unwrap(),
                    year: row.get(2).unwrap(),
                } 
            )
        }).expect("couldn't create 'month_iter'");

        return month_iter.map(|mon| mon.unwrap()).collect::<Vec<Month>>()[0];
    }

    fn is_expense(&self, amount: isize) -> u8 {
        if amount < 0 {
            return 1;
        }
        return 0;
    }
}


#[derive(Parser, Debug)]
pub struct LineupArgs {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand, Debug)]
pub enum Action {
    Add(AddCommand),
    Show(ShowCommand),
    DeleteStatic(DeleteStatic),
    ShowStatics,
}


#[derive(Args, Debug)]
pub struct AddCommand {
    #[clap(subcommand)]
    add_type: AddType,
}

#[derive(Subcommand, Debug)]
pub enum AddType {
    Entry(NewEntry),
    Static(NewStatic),
}

#[derive(Args, Debug)]
pub struct NewEntry {
    pub name: String,
    pub amount: isize,
    #[clap(default_value="0")]
    pub month: u32,
    #[clap(default_value="0")]
    pub year: i32,
}

#[derive(Args, Debug)]
pub struct NewStatic {
    pub name: String,
    pub amount: isize,
}


#[derive(Args, Debug)]
pub struct ShowCommand {
    #[clap(default_value="0")]
    pub month: u32,
    #[clap(default_value="0")]
    pub year: i32,
}


#[derive(Args, Debug)]
pub struct DeleteStatic {
    pub id: usize,
}


fn get_date() -> (u32, i32) {
    let now = Utc::now();

    return (now.month(), now.year());
}


fn add(add_type: &AddType, data: Database) {
    let current = get_date();

    match add_type {
        AddType::Entry(new_entry) => {
            match new_entry.year {
                0 => match new_entry.month {
                    0 => data.new_entry(&new_entry.name, new_entry.amount, current.0, current.1),
                    _ => data.new_entry(&new_entry.name, new_entry.amount, new_entry.month, current.1),
                },
                _ => match new_entry.month {
                    0 => data.new_entry(&new_entry.name, new_entry.amount, current.0, new_entry.year),
                    _ => data.new_entry(&new_entry.name, new_entry.amount, new_entry.month, new_entry.year),
                },
            }
            println!("\n{} Added new Entry: {}\n", "[LineUp]".red(), new_entry.name);
        },
        AddType::Static(new_static) => {
            data.new_static(&new_static.name, new_static.amount);
            println!("\n{} Added new Static: {}\n", "[LineUp]".red(), new_static.name);
        },
    }
}

fn show(month: u32, year: i32, data: Database) {
    let current = get_date();

    match year {
        0 => {
            match month {
                0 => display_month(current.0, current.1, data),
                _ => display_month(month, current.1, data),
            }
        },
        _ => {
            match month {
                0 => display_month(current.0, year, data),
                _ => display_month(month, year, data),
            }
        },
    }
}

fn display_month(month: u32, year: i32, data: Database) {
    let entrys = data.get_entrys(month, year);
    let mut sum_e = 0;

    println!("\n{}", "[LineUp]\n".red());

    for x in entrys {
        sum_e = sum_e + x.amount;
        if x.amount < 0 {
            println!("      {}{}", x.name.with_exact_width(25), x.amount.to_string().pad_to_width_with_alignment(10, Alignment::Right).red());
        } else {
            println!("      {}{}", x.name.with_exact_width(25), x.amount.to_string().pad_to_width_with_alignment(10, Alignment::Right).green());
        }
    }

    println!("      {}", "".pad_to_width_with_char(35, '-'));

    if sum_e < 0 {
        println!("      Summe{}\n", sum_e.to_string().pad_to_width_with_alignment(30, Alignment::Right).red());
    } else {
        println!("      Summe{}\n", sum_e.to_string().pad_to_width_with_alignment(30, Alignment::Right).green());
    }
}

fn show_statics(data: Database) {
    let statics = data.get_statics();

    println!("\n{}", "[LineUp]\n".red());

    for x in statics {
        if x.amount < 0 {
            println!("      {}{}{}", x.id.to_string().with_exact_width(5), x.name.with_exact_width(25), x.amount.to_string().pad_to_width_with_alignment(10, Alignment::Right).red())
        } else {
            println!("      {}{}{}", x.id.to_string().with_exact_width(5), x.name.with_exact_width(25), x.amount.to_string().pad_to_width_with_alignment(10, Alignment::Right).green())
        }
    }

    println!("\n");
}

fn delete_static(id: usize, data: Database) {
    data.delete_static(id);

    println!("\n{} Deleted static value with id = {}.\n", "[LineUp]".red(), id);
}


fn main() {
    let data = Database::open();
    let args = LineupArgs::parse().action;

    match &args {
        Action::Add(add_command) => add(&add_command.add_type, data),
        Action::Show(show_command) => show(show_command.month, show_command.year, data),
        Action::DeleteStatic(d_static) => delete_static(d_static.id, data),
        Action::ShowStatics => show_statics(data),
    }
}
