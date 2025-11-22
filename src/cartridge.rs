#[derive(Debug, Clone, Copy)]
pub enum CartridgeType {
    RomOnly,
    Mbc1,
    Mbc1Ram,
}

impl TryFrom<u8> for CartridgeType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::RomOnly),
            1 => Ok(Self::Mbc1),
            2 => Ok(Self::Mbc1Ram),
            _ => Err(value),
        }
    }
}
