use structopt::StructOpt;
use x11::{xlib, xtest};

use std::ffi::CString;

type Error = Box<dyn std::error::Error>;

#[derive(StructOpt)]
struct Opts {
    /// The X11 name of the key to be pressed
    #[structopt(
        long,
        index(1),
        required_unless_one(&["mouse", "dump"]),
        conflicts_with_all(&["mouse", "dump"])
    )]
    key: Option<String>,
    #[structopt(
        name = "mouse",
        long,
        required_unless_one(&["key", "dump"]),
        conflicts_with_all(&["key", "dump"])
    )]

    /// The integer index of the mouse button to be pressed
    mouse_button: Option<u32>,
    #[structopt(long)]

    /// If provided a release event will be sent instead of a press event
    release: bool,
    #[structopt(
        long,
        required_unless_one(&["key", "mouse"]),
        conflicts_with_all(&["key", "mouse"])
    )]

    /// Print a list of all the available keys in the current keymap
    dump: bool,
}

fn main() -> Result<(), Error> {
    let opts = Opts::from_args();

    let mut display = Display::new()?;

    if opts.dump {
        display.dump();
    } else {
        let state = if opts.release {
            ButtonState::Released
        } else {
            ButtonState::Pressed
        };

        match (&opts.key, &opts.mouse_button) {
            (Some(key), None) => {
                display.send_key(key, state)?;
            }
            (None, Some(mouse)) => {
                display.send_button(*mouse, state)?;
            }
            _ => unreachable!("Either <mouse> or <key> must be suplied"),
        }
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
}

pub struct Display {
    display: *mut xlib::Display,
}

impl Display {
    pub fn new() -> Result<Self, Error> {
        let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            Err("Could not acquire XDisplay".into())
        } else {
            let display = Display { display };

            Ok(display)
        }
    }

    fn keycode_range(&mut self) -> std::ops::RangeInclusive<i32> {
        let mut min_keycode = 0;
        let mut max_keycode = 0;
        unsafe {
            xlib::XDisplayKeycodes(self.display, &mut min_keycode, &mut max_keycode);
        }
        min_keycode..=max_keycode
    }

    pub fn dump(&mut self) {
        unsafe {
            for n in self.keycode_range() {
                let keysym = xlib::XKeycodeToKeysym(self.display, n as u8, 0);
                let name = xlib::XKeysymToString(keysym);
                if name.is_null() {
                    continue;
                }
                let name = std::ffi::CStr::from_ptr(name);

                if let Ok(name) = name.to_str() {
                    println!("{}", name);
                }
            }
        }
    }

    pub fn send_key(&mut self, key: &str, state: ButtonState) -> Result<(), Error> {
        let c_key = CString::new(key)?;
        let keysym = unsafe { xlib::XStringToKeysym(c_key.as_ptr()) };
        if keysym as i32 == xlib::NoSymbol {
            return Err(format!("Key '{}' not found", key).into());
        }
        let keycode = unsafe { xlib::XKeysymToKeycode(self.display, keysym) };
        if !self.keycode_range().contains(&(keycode as i32)) {
            return Err(format!("Keycode for keysym of '{}' not found", key).into());
        }
        let pressed = match state {
            ButtonState::Pressed => 1,
            ButtonState::Released => 0,
        };
        unsafe {
            xtest::XTestFakeKeyEvent(self.display, keycode as u32, pressed, 0);
        }

        self.flush();

        Ok(())
    }

    pub fn send_button(&mut self, button: u32, state: ButtonState) -> Result<(), Error> {
        if button > 10 {
            return Err(format!("Mouse button '{}' out of range", button).into());
        }

        let pressed = match state {
            ButtonState::Pressed => 1,
            ButtonState::Released => 0,
        };
        unsafe {
            xtest::XTestFakeButtonEvent(self.display, button, pressed, 0);
        }

        self.flush();

        Ok(())
    }

    pub fn flush(&mut self) {
        unsafe {
            xlib::XFlush(self.display);
        }
    }
}

impl std::ops::Drop for Display {
    fn drop(&mut self) {
        unsafe { xlib::XCloseDisplay(self.display) };
    }
}
