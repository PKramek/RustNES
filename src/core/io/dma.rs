#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OamDmaPort {
    last_page: Option<u8>,
    pending: bool,
}

impl OamDmaPort {
    pub fn request(&mut self, page: u8) {
        self.last_page = Some(page);
        self.pending = true;
    }

    pub fn last_page(&self) -> Option<u8> {
        self.last_page
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn clear(&mut self) {
        self.pending = false;
    }
}
