//! Code in its colors (D95): each piece the colorer gave a role, painted in
//! the theme's color for that role.

use super::*;
use crate::code_colors::Colored;
use eframe::egui::text::{ByteIndex, LayoutSection};

/// `colored`'s code laid out in `font`, plain code in `color`.
pub(super) fn code_job(colored: &Colored, font: FontId, color: Color32, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    let format = |color| TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };
    let text = colored.text.as_str();
    let mut at = 0;
    for (range, role) in &colored.pieces {
        if range.start > at {
            job.append(&text[at..range.start], 0.0, format(color));
        }
        let shade = theme::code_color(*role, dark).unwrap_or(color);
        job.append(&text[range.clone()], 0.0, format(shade));
        at = range.end;
    }
    if at < text.len() {
        job.append(&text[at..], 0.0, format(color));
    }
    job
}

/// Paints the roles of `colored` into `job`, which lays out the start of
/// its code, as a card shows it; words found by a search keep their look.
pub(super) fn color_job(job: &mut LayoutJob, colored: &Colored, dark: bool) {
    if !colored.text.starts_with(job.text.as_str()) {
        return;
    }
    let mut sections = Vec::with_capacity(job.sections.len());
    for section in std::mem::take(&mut job.sections) {
        if section.format.background != Color32::TRANSPARENT {
            sections.push(section);
            continue;
        }
        let (start, end) = (section.byte_range.start.0, section.byte_range.end.0);
        let mut part = |from: usize, to: usize, color: Option<Color32>| {
            let mut format = section.format.clone();
            if let Some(color) = color {
                format.color = color;
            }
            sections.push(LayoutSection {
                leading_space: if from == start {
                    section.leading_space
                } else {
                    0.0
                },
                byte_range: ByteIndex(from)..ByteIndex(to),
                format,
            });
        };
        let mut at = start;
        for (range, role) in &colored.pieces {
            if range.end <= at {
                continue;
            }
            if range.start >= end {
                break;
            }
            let (from, to) = (range.start.max(at), range.end.min(end));
            if from > at {
                part(at, from, None);
            }
            part(from, to, theme::code_color(*role, dark));
            at = to;
        }
        if at < end {
            part(at, end, None);
        }
    }
    job.sections = sections;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_colors::Role;

    fn python() -> Colored {
        // "def go(): # x"
        Colored {
            text: "def go(): # x".into(),
            pieces: vec![
                (0..3, Role::Keyword),
                (4..6, Role::Function),
                (10..13, Role::Comment),
            ],
        }
    }

    fn pieces(job: &LayoutJob) -> Vec<(&str, Color32)> {
        job.sections
            .iter()
            .map(|s| {
                (
                    &job.text[s.byte_range.start.0..s.byte_range.end.0],
                    s.format.color,
                )
            })
            .collect()
    }

    #[test]
    fn code_shows_each_piece_in_its_roles_color() {
        // A color no role has, so plain pieces stay apart from the others.
        let plain = Color32::from_rgb(1, 2, 3);
        let job = code_job(&python(), FontId::monospace(13.0), plain, true);
        assert_eq!(job.text, "def go(): # x");
        let keyword = theme::code_color(Role::Keyword, true).unwrap();
        let function = theme::code_color(Role::Function, true).unwrap();
        let comment = theme::code_color(Role::Comment, true).unwrap();
        assert_eq!(
            pieces(&job),
            [
                ("def", keyword),
                (" ", plain),
                ("go", function),
                ("(): ", plain),
                ("# x", comment)
            ]
        );
    }

    /// A card's text is the start of the code, with found words marked;
    /// the marks stay as they are and the rest takes the roles' colors.
    #[test]
    fn a_cards_code_keeps_the_found_words_marked() {
        let (plain, hit, ground) = (
            Color32::from_rgb(1, 2, 3),
            Color32::WHITE,
            Color32::DARK_BLUE,
        );
        let format = |color, background| TextFormat {
            color,
            background,
            ..Default::default()
        };
        let mut job = LayoutJob::default();
        job.append("def ", 0.0, format(plain, Color32::TRANSPARENT));
        job.append("go", 0.0, format(hit, ground));
        job.append("(): #", 0.0, format(plain, Color32::TRANSPARENT));
        color_job(&mut job, &python(), false);
        let keyword = theme::code_color(Role::Keyword, false).unwrap();
        let comment = theme::code_color(Role::Comment, false).unwrap();
        assert_eq!(
            pieces(&job),
            [
                ("def", keyword),
                (" ", plain),
                ("go", hit),
                ("(): ", plain),
                ("#", comment)
            ]
        );
        // Text that isn't the code's start stays as it was.
        let mut other = LayoutJob::default();
        other.append("something else", 0.0, format(plain, Color32::TRANSPARENT));
        color_job(&mut other, &python(), false);
        assert_eq!(pieces(&other), [("something else", plain)]);
    }
}
