const LEAD_SURROGATE_MIN: u16 = 0xd800;
const TRAIL_SURROGATE_MAX: u16 = 0xdfff;
const CODE_POINT_MAX: u32 = 0x0010ffff;

macro_rules! mask8 {
    ($oc:expr) => {
        (0xff & $oc) as u8
    };
}

macro_rules! is_trail {
    ($oc:expr) => {
        (mask8!($oc) >> 6) == 0x2
    };
}

macro_rules! is_surrogate {
    ($cp:expr) => {{
        ($cp >= LEAD_SURROGATE_MIN && $cp <= TRAIL_SURROGATE_MAX)
    }};
}

macro_rules! is_code_point_valid {
    ($cp:expr) => {{
        ($cp <= CODE_POINT_MAX && !is_surrogate!($cp as u16))
    }};
}

#[inline]
const fn sequence_length(lead_byte: u8) -> usize {
    if lead_byte < 0x80 {
        1
    } else if (lead_byte >> 5) == 0x6 {
        2
    } else if (lead_byte >> 4) == 0xe {
        3
    } else if (lead_byte >> 3) == 0x1e {
        4
    } else {
        0
    }
}

#[inline]
const fn is_overlong_sequence(cp: u32, length: usize) -> bool {
    if cp < 0x80 {
        if length != 1 {
            return true;
        }
    } else if cp < 0x800 {
        if length != 2 {
            return true;
        }
    } else if cp < 0x10000 {
        if length != 3 {
            return true;
        }
    }
    false
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UtfError {
    NotEnoughRoom,
    InvalidLead,
    IncompleteSequence,
    OverlongSequence,
    InvalidCodePoint,
}

#[inline]
fn get_next_byte<'a, I>(it: &mut I) -> Result<&'a u8, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    it.next().ok_or(UtfError::NotEnoughRoom)
}

#[inline]
fn is_trail(byte: &u8) -> Result<&u8, UtfError> {
    if is_trail!(byte) {
        Ok(byte)
    } else {
        Err(UtfError::IncompleteSequence)
    }
}

#[inline]
fn get_sequence_1<'a, I>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    get_next_byte(it).map(|byte| *byte as u32)
}

#[inline]
fn get_sequence_2<'a, I>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(((code_point << 6) & 0x7ff) + ((byte & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
fn get_sequence_3<'a, I>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(((code_point << 12) & 0xffff) + (((*byte as u32) << 6) & 0xfff)))?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + ((byte & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
fn get_sequence_4<'a, I>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it).and_then(is_trail).and_then(|byte| {
        Ok(((code_point << 18) & 0x1fffff) + (((*byte as u32) << 12) & 0x3ffff))
    })?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + (((*byte as u32) << 6) & 0xfff)))?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + (((byte) & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
pub fn validate_next<'a, I>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator<Item = &'a u8>,
{
    let mut it = it.peekable();
    let lead = it.peek().ok_or(UtfError::InvalidLead)?;
    let length = sequence_length(**lead);
    let code_point = match length {
        0 => Err(UtfError::InvalidLead),
        1 => get_sequence_1(&mut it),
        2 => get_sequence_2(&mut it),
        3 => get_sequence_3(&mut it),
        4 => get_sequence_4(&mut it),
        _ => unreachable!(),
    }
    .and_then(|code_point| {
        if is_code_point_valid!(code_point) {
            if !is_overlong_sequence(code_point, length) {
                Ok(code_point)
            } else {
                Err(UtfError::OverlongSequence)
            }
        } else {
            Err(UtfError::InvalidCodePoint)
        }
    });
    code_point
}

#[cfg(test)]
mod test_core {
    use log::info;

    use super::*;

    fn init_logger() {
        let _ = env_logger::builder()
            // .filter_level(log::LevelFilter::max())
            .format_timestamp(None)
            // .is_test(true)
            .try_init();
    }

    #[test]
    fn test_validate_next_1() {
        init_logger();
        let input = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
        let mut it = input.as_bytes().iter();
        for c in input.chars() {
            info!("try valide {}", c);
            info!("his code {:#b}", c as u32);
            let r = validate_next(&mut it).unwrap();
            info!("val code {:#b}", r as u32);
            let r = unsafe { char::from_u32_unchecked(r) };
            assert_eq!(c, r);
        }
        assert!(validate_next(&mut it).is_err())
    }

    #[test]
    fn test_validate_next_2() {
        init_logger();
        let input = "¡¢£¤¥¦§¨©ª«¬­®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔ";
        let mut it = input.as_bytes().iter();
        for c in input.chars() {
            info!("try valide {}", c);
            info!("his code {:#b}", c as u32);
            let r = validate_next(&mut it).unwrap();
            info!("val code {:#b}", r as u32);
            let r = unsafe { char::from_u32_unchecked(r) };
            assert_eq!(c, r);
        }
        assert!(validate_next(&mut it).is_err())
    }

    #[test]
    fn test_validate_next_3() {
        init_logger();
        let input = "ขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรฤลฦวศษสหฬอฮฯะัาำิีึืฺุู";
        let mut it = input.as_bytes().iter();
        for c in input.chars() {
            info!("try valide {}", c);
            info!("his code {:#b}", c as u32);
            let r = validate_next(&mut it).unwrap();
            info!("val code {:#b}", r as u32);
            let r = unsafe { char::from_u32_unchecked(r) };
            assert_eq!(c, r);
        }
        assert!(validate_next(&mut it).is_err())
    }

    #[test]
    fn test_validate_next_4() {
        init_logger();
        let input = "😀𒀀𒀁𒀂𒀃𒀄𒀅𒀆𒀇𒀈𒀉𒀊𒀋𒀌𒀍𒀎𒀏";
        let mut it = input.as_bytes().iter();
        for c in input.chars() {
            info!("try valide {}", c);
            info!("his code {:#b}", c as u32);
            let r = validate_next(&mut it).unwrap();
            info!("val code {:#b}", r as u32);
            let r = unsafe { char::from_u32_unchecked(r) };
            assert_eq!(c, r);
        }
        assert!(validate_next(&mut it).is_err())
    }
}
