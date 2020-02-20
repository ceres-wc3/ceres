use std::collections::HashMap;
use std::iter::Peekable;
use std::str;

use num_traits::Num;

use atoi::atoi;

use crate::parser::crlf::Lines;

fn strip_field(src: &[u8]) -> (&[u8], &[u8]) {
    let mut i = 0;
    while i < src.len() && src[i] != b';' {
        i += 1;
    }

    if i == src.len() {
        (&src[..], &[])
    } else {
        (&src[0..i], &src[i + 1..src.len()])
    }
}

#[derive(Debug)]
pub enum Value<'src> {
    Str(&'src str),
    Num(&'src str),
    Empty,
}

impl<'src> Value<'src> {
    fn from_slice(src: &'src [u8]) -> Value<'src> {
        if src.is_empty() {
            Value::Empty
        } else if src[0] == b'"' {
            Value::Str(Self::parse_string(src))
        } else {
            Value::Num(str::from_utf8(src).unwrap())
        }
    }

    fn parse_string(src: &'src [u8]) -> &'src str {
        let mut i = 1;

        while i < src.len() && src[i] != b'"' {
            i += 1;
        }

        str::from_utf8(&src[1..i]).unwrap()
    }

    pub fn as_inner(&self) -> Option<&'src str> {
        match self {
            Value::Str(value) => Some(value),
            Value::Num(value) => Some(value),
            Value::Empty => None,
        }
    }

    pub fn as_str(&self) -> Option<&'src str> {
        match self {
            Value::Str(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_num<N: Num>(&self) -> Option<N> {
        match self {
            Value::Num(value) => Some(N::from_str_radix(value, 10).ok().unwrap()),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Field<'src> {
    Col(u32),
    Row(u32),
    Value(&'src [u8]),
    Unknown,
}

impl<'src> Field<'src> {
    fn from_slice(src: &'src [u8]) -> (Option<Field<'src>>, &'src [u8]) {
        let (field, rest) = strip_field(src);

        if field.is_empty() {
            return (None, rest);
        }

        let field = match field[0] {
            b'X' => atoi(&field[1..]).map(Field::Col),
            b'Y' => atoi(&field[1..]).map(Field::Row),
            b'K' => Some(Field::Value(&field[1..])),
            _ => Some(Field::Unknown),
        };

        (field, rest)
    }
}

#[derive(Debug)]
pub struct Cell<'src> {
    column: u32,
    row:    Option<u32>,
    value:  Value<'src>,
}

impl<'src> Cell<'src> {
    fn from_bytes(src: &'src [u8]) -> Option<Cell<'src>> {
        let (field, mut rest) = strip_field(src);

        let mut cell = if field == b"C" {
            Cell {
                column: 0,
                row:    None,
                value:  Value::Empty,
            }
        } else {
            return None;
        };

        while !rest.is_empty() {
            let (field, new_rest) = Field::from_slice(rest);
            rest = new_rest;

            if let Some(field) = field {
                cell.apply_field(field);
            }
        }

        Some(cell)
    }

    fn apply_field(&mut self, field: Field<'src>) {
        match field {
            Field::Col(field_col) => self.column = field_col,
            Field::Row(field_row) => self.row = Some(field_row),
            Field::Value(field_value) => self.value = Value::from_slice(field_value),
            _ => {}
        }
    }

    pub fn value(&self) -> &Value<'src> {
        &self.value
    }

    pub fn column(&self) -> u32 {
        self.column
    }
}

#[derive(Debug)]
struct Parser<'src> {
    lines: Lines<'src>,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src [u8]) -> Parser<'src> {
        Parser {
            lines: Lines::new(source),
        }
    }

    pub fn next_record(&mut self) -> Option<Cell<'src>> {
        while let Some(line) = self.lines.next() {
            if let Some(cell) = Cell::from_bytes(line) {
                return Some(cell);
            }
        }

        None
    }
}

struct Cells<'src> {
    parser: Parser<'src>,
}

impl<'src> Cells<'src> {
    fn into_rows(self) -> Rows<'src> {
        Rows {
            cells: self.peekable(),
        }
    }
}

impl<'src> Iterator for Cells<'src> {
    type Item = Cell<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_record()
    }
}

impl<'src> IntoIterator for Parser<'src> {
    type IntoIter = Cells<'src>;
    type Item = Cell<'src>;

    fn into_iter(self) -> Self::IntoIter {
        Cells { parser: self }
    }
}

#[derive(Debug)]
pub struct Row<'src> {
    pub position: u32,
    pub cells:    Vec<Cell<'src>>,
}

struct Rows<'src> {
    cells: Peekable<Cells<'src>>,
}

impl<'src> Iterator for Rows<'src> {
    type Item = Row<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cells.peek().is_none() {
            None
        } else {
            let row_start = self.cells.find(|r| r.row.is_some());

            if let Some(row_start) = row_start {
                let mut cells = Vec::new();
                let position = row_start.row.unwrap();
                cells.push(row_start);

                while let Some(peeked) = self.cells.peek() {
                    if peeked.row.is_some() {
                        break;
                    } else {
                        cells.push(self.cells.next().unwrap())
                    }
                }

                Some(Row { position, cells })
            } else {
                None
            }
        }
    }
}

#[derive(Clone)]
pub struct Legend<'src> {
    name_to_idx: HashMap<&'src str, u32>,
    idx_to_name: HashMap<u32, &'src str>,
}

impl<'src> Legend<'src> {
    fn new(row: Row<'src>) -> Legend<'src> {
        let mut name_to_idx = HashMap::default();
        let mut idx_to_name = HashMap::default();

        for cell in row.cells {
            let name = cell.value.as_str();

            if let Some(name) = name {
                name_to_idx.insert(name, cell.column);
                idx_to_name.insert(cell.column, name);
            }
        }

        Legend {
            name_to_idx,
            idx_to_name,
        }
    }

    pub fn cell_by_name<'r>(&self, row: &'r Row<'src>, name: &str) -> Option<&'r Cell<'src>> {
        let col_idx = *self.name_to_idx.get(name)?;
        row.cells.iter().find(|c| c.column == col_idx)
    }

    pub fn name_by_cell(&self, cell: &Cell) -> Option<&'src str> {
        self.idx_to_name.get(&cell.column).copied()
    }
}

pub struct Table<'src> {
    rows:   Rows<'src>,
    legend: Legend<'src>,
}

impl<'src> Table<'src> {
    pub fn new(source: &'src [u8]) -> Option<Table<'src>> {
        let mut rows = Parser::new(source).into_iter().into_rows();
        let legend = Legend::new(rows.next()?);

        Some(Table { rows, legend })
    }

    pub fn next_row(&mut self) -> Option<Row<'src>> {
        self.rows.next()
    }

    pub fn has_next(&mut self) -> bool {
        self.rows.cells.peek().is_some()
    }

    pub fn legend(&self) -> Legend<'src> {
        self.legend.clone()
    }
}

pub fn read_row_str<'src>(row: &Row<'src>, legend: &Legend<'src>, name: &str) -> Option<&'src str> {
    legend
        .cell_by_name(row, name)
        .map(|r| r.value())
        .and_then(|r| r.as_str())
}

pub fn read_row_num<'src, N: Num>(row: &Row<'src>, legend: &Legend<'src>, name: &str) -> Option<N> {
    legend
        .cell_by_name(row, name)
        .map(|r| r.value())
        .and_then(|r| r.as_num())
}
