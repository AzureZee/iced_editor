use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    Application, Command, Element, Font, Length, Settings, Theme, executor, theme, widget::{button, column, container, horizontal_space, row, text, text_editor, tooltip}
};

fn main() -> iced::Result {
    Editor::run(Settings {
        fonts: vec![
            include_bytes!("../fonts/editor_icons.ttf")
                .as_slice()
                .into(),
        ],
        ..Settings::default()
    })
}

struct Editor {
    path: Option<PathBuf>,
    content: text_editor::Content,
    error: Option<Error>,
}
#[derive(Debug, Clone)]
enum Message {
    New,
    Open,
    Save,
    FileSaved(Result<PathBuf, Error>),
    Edit(text_editor::Action),
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
}

const NEW_TIP: &str = "new file";
const OPEN_TIP: &str = "open file";
const SAVE_TIP: &str = "save file";

impl Application for Editor {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let default_file = PathBuf::from(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR")));
        (
            Self {
                content: text_editor::Content::new(),
                error: None,
                path: None,
            },
            Command::perform(load_file(default_file), Message::FileOpened),
        )
    }

    fn title(&self) -> String {
        String::from("A cool editor!")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Edit(action) => {
                self.content.edit(action);
                self.error = None;
                Command::none()
            }
            Message::FileOpened(Ok((path, content))) => {
                self.path = Some(path);
                self.content = text_editor::Content::with(&content);
                Command::none()
            }
            Message::FileOpened(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
            Message::Open => Command::perform(pick_file(), Message::FileOpened),
            Message::New => {
                self.path = None;
                self.content = text_editor::Content::new();
                Command::none()
            }
            Message::Save => {
                let text = self.content.text();
                Command::perform(save_file(self.path.clone(), text), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                Command::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row!(
            action(new_icon(), NEW_TIP,Message::New),
            action(open_icon(),OPEN_TIP, Message::Open),
            action(save_icon(),SAVE_TIP, Message::Save),
        )
        .spacing(10);
        let input = text_editor(&self.content).on_edit(Message::Edit);
        let status_bar = {
            let status = if let Some(Error::IOFailed(error)) = self.error.as_ref() {
                text(error.to_string())
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("New file"),
                }
            };
            let position = {
                let (line, column) = self.content.cursor_position();
                text(format!("{}:{}", line + 1, column + 1))
            };

            row!(status, horizontal_space(Length::Fill), position)
        };
        container(column!(controls, input, status_bar))
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        self::Theme::Dark
    }
}

fn action<'a>(
    content: Element<'a, Message>,
    label: &str,
    on_press: Message,
) -> Element<'a, Message> {
    let btn = button(container(content).width(30).center_x())
        .on_press(on_press)
        .padding([5, 10]);
    tooltip(btn, label, tooltip::Position::FollowCursor).style(theme::Container::Box).into()
}

const SAVE_ICON: char = '\u{E800}';
const NEW_ICON: char = '\u{E801}';
const OPEN_ICON: char = '\u{F115}';
fn new_icon<'a, Message>() -> Element<'a, Message> {
    icon(NEW_ICON)
}
fn save_icon<'a, Message>() -> Element<'a, Message> {
    icon(SAVE_ICON)
}
fn open_icon<'a, Message>() -> Element<'a, Message> {
    icon(OPEN_ICON)
}
fn icon<'a, Message>(code_point: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("editor_icons");

    text(code_point).font(ICON_FONT).into()
}

async fn pick_file() -> Result<(PathBuf, Arc<String>), Error> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("Choose a text file...")
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;
    load_file(handle.path().to_path_buf()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), Error> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| Error::IOFailed(error.kind()))?;
    Ok((path, content))
}

async fn save_file(path: Option<PathBuf>, text: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path
        && path.is_file()
    {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .set_title("Create a file")
            .save_file()
            .await
            .ok_or(Error::DialogClosed)
            .map(|handle| handle.path().to_path_buf())?
    };
    tokio::fs::write(&path, &text)
        .await
        .map_err(|error| Error::IOFailed(error.kind()))?;
    Ok(path)
}

#[derive(Debug, Clone)]
enum Error {
    DialogClosed,
    IOFailed(io::ErrorKind),
}
