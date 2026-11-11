//! Text → phonemes for the app: Russian (Cyrillic) words through [`RussianG2P`], the rest through
//! [`EnglishG2P`]. A text with any Cyrillic is read as Russian, its Latin words still in English.

use super::Segment;
use super::english::{EnglishG2P, Phrase};
use super::russian::{RussianG2P, has_cyrillic};
use super::syllables::WordSource;

#[derive(Clone, Debug, Default)]
pub struct G2P {
    pub en: EnglishG2P,
    pub ru: RussianG2P,
}

impl G2P {
    pub fn new() -> G2P {
        G2P::default()
    }

    /// Text → phrases of segments.
    pub fn to_phrases(&self, text: &str) -> Vec<Phrase> {
        if has_cyrillic(text) { self.ru.to_phrases(text, &self.en) } else { self.en.to_phrases(text) }
    }

    /// One word → segments with lexical stress. `reduce`: speech (sung otherwise).
    pub fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        if has_cyrillic(w) { self.ru.word(w, reduce) } else { self.en.word(w, reduce) }
    }
}

impl WordSource for G2P {
    fn word(&self, w: &str, reduce: bool) -> Vec<Segment> {
        G2P::word(self, w, reduce)
    }
}
