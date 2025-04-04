use crate::tui;

use color_eyre::{
    eyre::WrapErr, Result
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use rand::{thread_rng, Rng};
use ratatui::{
    prelude::*, 
    style::Color, 
    widgets::{block::*, Paragraph, *}
};

use std::env;

use crate::read_write::*;

#[derive(Debug, Default)]
pub struct App {
    pub score: u64,
    pub highscore: u64,
    exit: bool,
    on_pause: bool,
    dead: bool,
    grid: Grid,
    won: bool,
    ignore_win: bool
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
        where
            Self: Sized {

                let instructions = Title::from(Line::from(vec![
                    " move:".bold(),
                    " <arrows> ".bold(),
                    " exit:".bold(),
                    " <q> ".bold(),
                    " restart:".bold(),
                    " <Enter> ".bold()
                ]));

                let block = Block::default()
                    .borders(Borders::NONE)
                    .title(Title::from(" 2048 ".bold())
                        .alignment(Alignment::Center)
                        .position(Position::Top))
                    .title(instructions
                        .alignment(Alignment::Center)
                        .position(Position::Bottom))
                    .bg(Color::Black);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)].as_ref())
                    .split(area.inner(&Margin::new(32, 2)));

                Paragraph::new(Line::from(self.score.to_string()))
                    .alignment(Alignment::Left)
                    .block(block.clone())
                    .render(area, buf);

                Paragraph::new(Line::from(self.highscore.to_string()))
                    .alignment(Alignment::Right)
                    .block(block.clone())
                    .render(area, buf);
                

                if !self.dead {
                    for (i, chunk) in chunks.iter().enumerate() {
                        let inner_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)].as_ref())
                            .split(*chunk);
    
                        for (j, inner_chunk) in inner_chunks.iter().enumerate() {
                            let cell_block = Block::default()
                                .borders(Borders::ALL)
                                .fg(Color::White)
                                .bg(self.grid.fields[i * 4 + j].as_ref().unwrap().get_color());
    
                            // Render the block
                            cell_block.render(*inner_chunk, buf);
    
                            // Write the number inside the cell
                            let x = inner_chunk.x + (inner_chunk.width / 2) - 1;
                            let y = inner_chunk.y + (inner_chunk.height / 2);
                            buf.set_string(x, y, format!("{}", self.grid.fields[i * 4 + j].as_ref().unwrap().val), Style::default().fg(Color::Black));
                        }   
                    }
                }
                else {
                    Paragraph::new(Line::from(" dead ".bold().red()))
                        .centered()
                        .block(block.clone())
                        .render(area, buf);
                }

                if self.won {
                    Paragraph::new(Line::from(vec![Span::from(" Congratulations you won |".bold()), Span::from(" restart: <Enter>, continue: <c>".bold())]))
                        .centered()
                        .block(block.clone())
                        .render(area, buf);
                }
    }   
}

impl App {

    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
            if self.exit {
                break;
            } 
            if self.on_pause || self.dead {
                continue;
            }
            self.highscore();
            if self.ignore_win {
                self.reset_max();
            }
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn highscore(&mut self) {
        if self.score > self.highscore {
            self.highscore = self.score;
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event).wrap_err_with(|| {
                    format!("handling key event failed: \n{key_event:#?}")
                })
            }
           _ => Ok(())
        }
    }

    pub fn new() -> Result<Self> {
        let app = App {
            score: 0,
            highscore: 0,
            exit: false,
            dead: false,
            on_pause: false,
            grid: Grid::new(),
            won: false,
            ignore_win: false
        };
        Ok(app)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Esc => self.pause()?,
            KeyCode::Enter => self.restart()?,
            KeyCode::Right => self.move_right()?,
            KeyCode::Left => self.move_left()?,
            KeyCode::Up => self.move_up()?,
            KeyCode::Down => self.move_down()?,
            KeyCode::Char('c') => self.ignore_win = true,
            _ => {}
        }
        self.check_for_win();
        Ok(())
    }

    fn restart(&mut self) -> Result<()> {

        if self.dead {
            let path_to_self = env::current_exe()?;
            let path = path_to_self
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p|p.parent())
                .map(|p|p.join("Highscore.bin"))
                .unwrap();
            save(&path, self.highscore)?;
            
            let num = read(&path)?;

            self.highscore = num;
            self.score = 0;
            self.on_pause = false;
            self.dead = false;
            self.grid = Grid::new();
        }

        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn pause(&mut self) -> Result<()> {
        if self.on_pause {
            self.on_pause = false;
        }
        else {
            self.on_pause = true;
        }
        Ok(())
    }

    fn is_dead(&mut self) -> Result<()> {
        if !self.dead {
            self.dead = true;
        }
        Ok(())
    }

    fn move_left(&mut self) -> Result<()>{
        self.grid.move_vals(3, &mut self.score)?;
        self.new_pieces()?;
        Ok(())
    }

    fn move_right(&mut self) -> Result<()> {
        self.grid.move_vals(1, &mut self.score)?;
        self.new_pieces()?;
        Ok(())
    }

    fn move_down(&mut self) -> Result<()> {
        self.grid.move_vals(2, &mut self.score)?;
        self.new_pieces()?;
        Ok(())
    }

    fn move_up(&mut self) -> Result<()> {
        self.grid.move_vals(0, &mut self.score)?;
        self.new_pieces()?;
        Ok(())
    }

    fn new_pieces(&mut self) -> Result<()> {
        let mut rng = thread_rng();
        let all_full = self.grid.fields.iter().all(|field| field.as_ref().unwrap().val != 0);
        loop {
            for field in self.grid.fields.iter_mut() {
                let rand = rng.gen_range(0.0..1.0);
                if field.as_ref().unwrap().val == 0 && rand < 1.0 / 16.0 {
                    let rand_2 = rng.gen_range(0.0..1.0);
                    if rand_2 < 0.15 {
                        field.as_mut().unwrap().val = 2;
                        return  Ok(());
                    }
                    else {
                        field.as_mut().unwrap().val = 2;
                        return Ok(());
                    }
                }
                if rand < 0.1 && all_full {
                    self.is_dead()?;
                    return Ok(());
                }
            }
        }
    }

    fn check_for_win(&mut self){
        if self.ignore_win {
            self.won = false;
            return;
        }
        self.won = self.grid.get_state();
    }

    fn reset_max(&mut self) {
        let _: () = self.grid.fields.iter_mut().map(|field|{
            if field.as_ref().unwrap().val >= 2048 {
                field.as_mut().unwrap().val = 0;
            }
        }).collect();
    }

}

#[derive(Debug, Default, Clone)]
struct Grid {
    fields: Vec<Option<Field>>
}

impl Grid {

    fn move_vals(&mut self, direction: usize, score: &mut u64) -> Result<()> {
        if ![0,1,2,3].iter().any(|val| val == &direction) {
            println!("exit");
            return Ok(());
        }

        for _ in 0..4{
            for i in 0..self.fields.len() {
                let _ = recursive_merge(&Option::from(i), direction, &mut self.fields, score);
            }
        }
        for field in self.fields.iter_mut() {
            field.as_mut().unwrap().reset_blocker();
        }
        Ok(())
    }

    fn new() -> Self {
        let mut  grid = Grid {
            fields: vec![Option::from(Field::new()); 16],
        };

        Self::init_neighbours(&mut grid);
        Self::init_grid(&mut grid);

        grid
    }

    fn init_grid(grid: &mut Self) {
        let mut rng = thread_rng();
        for field in grid.fields.iter_mut() {
            let rand = rng.gen_range(0.0..1.0);
            if field.as_ref().unwrap().val == 0 && rand < 0.1 {
                field.as_mut().unwrap().val = 0;
            }
        }
        if grid.fields.iter().all(|field| field.as_ref().unwrap().val == 0) {
            let random_index = rng.gen_range(0..=15);
                grid.fields[random_index].as_mut().unwrap().val = 2;
        }
    }

    fn init_neighbours(grid: &mut Self) {
        for (i, field) in grid.fields.iter_mut().enumerate() {
            let top: Option<usize>;
            let right: Option<usize>;
            let bot: Option<usize>;
            let left: Option<usize>;
            if i < 4 {
                top = Option::from(None);
            }
            else {
                top = Option::from(i - 4);
            }
            if [0, 4, 8, 12].iter().any(|val| val == &i) {
                left = Option::from(None);
            }
            else {
                left = Option::from(i - 1);
            }
            if i > 11 {
                bot = Option::from(None);
            }
            else {
                bot = Option::from(i + 4);
            }
            if [3, 7, 11, 15].iter().any(|val| val == &i) {
                right = Option::from(None);
            }
            else {
                right = Option::from(i + 1);
            }
            field.as_mut().unwrap().neighbours = vec![top, right, bot, left];
        }
    }

    fn get_state(&self) -> bool {
        self.fields.iter().any(|field| field.as_ref().unwrap().val == 2048)
    }

}

#[derive(Debug, Default, Clone)]
struct Field {
    val: u64,
    neighbours: Vec<Option<usize>>, // top right bottom left
    has_merged: bool
}

impl Field {
    fn new() -> Self {
        Field {
            val: 0,
            neighbours: vec![],
            has_merged: false
        }
    }

    fn check_for_merge(&self, next_val: u64) -> bool {
        if (self.val == next_val || self.val == 0) && !self.has_merged {
            return true;
        }
        false
    }

    fn merge(&mut self, moving: u64, score: &mut u64) {
        if self.val > 0 {
            self.has_merged = true;
            *score += self.val + moving;
        }
        self.val += moving;
    }

    fn reset_blocker(&mut self) {
        self.has_merged = false;
    }

    fn get_color(&self) -> Color {
        match self.val {
            0 => return Color::Black,
            2 => return Color::LightYellow,
            4 => return Color::White,
            8 => return Color::Blue,
            16 => return Color::Green,
            32 => return Color::Yellow,
            64 => return Color::Red,
            128 => return Color::Cyan,
            256 => return Color::LightMagenta,
            512 => return Color::Magenta,
            1024 => return Color::LightBlue,
            2024 => return  Color::Rgb(255, 0, 255),
            _ => return  Color::DarkGray,
        }
    }
}

fn recursive_merge(mv_field: &Option<usize>, direction: usize, fields: &mut Vec<Option<Field>>, score: &mut u64) -> Result<bool> {
    match mv_field {
        None => return Ok(false),
        Some(field) => {
            let next_index = &fields[*field].as_ref().unwrap().neighbours[direction].clone();
            let is_movable = recursive_merge(next_index, direction, fields, score)?;
            if !is_movable {
                return Ok(true);
            }
            let current_val = fields[*field].as_ref().unwrap().val.clone();
            let next_field = fields[next_index.unwrap()].as_mut().unwrap();
            let can_move = next_field.check_for_merge(current_val);
            if can_move {
                next_field.merge(current_val, score);
                fields[*field].as_mut().unwrap().val = 0;
            }
        }
    }
    Ok(true)
}
