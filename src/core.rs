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

#[derive(PartialEq)]
enum SeqLen {
    One,
    Two,
    Three,
    Four,
}

#[inline]
fn sequence_length(lead_byte: Option<u8>) -> Result<SeqLen, UtfError> {
    match lead_byte {
        Some(b) if b < 0x80 => Ok(SeqLen::One),
        Some(b) if (b >> 5) == 0x6 => Ok(SeqLen::Two),
        Some(b) if (b >> 4) == 0xe => Ok(SeqLen::Three),
        Some(b) if (b >> 3) == 0x1e => Ok(SeqLen::Four),
        _ => Err(UtfError::InvalidLead),
    }
}

#[inline]
fn is_overlong_sequence(cp: u32, length: SeqLen) -> bool {
    if cp < 0x80 {
        if length != SeqLen::One {
            return true;
        }
    } else if cp < 0x800 {
        if length != SeqLen::Two {
            return true;
        }
    } else if cp < 0x10000 {
        if length != SeqLen::Three {
            return true;
        }
    }
    false
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UtfError {
    EmptyInput,
    NotEnoughRoom,
    InvalidLead,
    IncompleteSequence,
    OverlongSequence,
    InvalidCodePoint,
}

#[inline]
fn get_next_byte<I, U>(it: &mut I) -> Result<u8, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    it.next()
        .map(|v| v.as_byte())
        .ok_or(UtfError::NotEnoughRoom)
}

#[inline]
fn is_trail(byte: u8) -> Result<u8, UtfError> {
    if is_trail!(byte) {
        Ok(byte)
    } else {
        Err(UtfError::IncompleteSequence)
    }
}

#[inline]
fn get_sequence_1<I, U>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    get_next_byte(it).map(|byte| byte as u32)
}

#[inline]
fn get_sequence_2<I, U>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(((code_point << 6) & 0x7ff) + ((byte & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
fn get_sequence_3<I, U>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(((code_point << 12) & 0xffff) + (((byte as u32) << 6) & 0xfff)))?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + ((byte & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
fn get_sequence_4<I, U>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    let code_point = get_sequence_1(it)?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(((code_point << 18) & 0x1fffff) + (((byte as u32) << 12) & 0x3ffff)))?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + (((byte as u32) << 6) & 0xfff)))?;
    let code_point = get_next_byte(it)
        .and_then(is_trail)
        .and_then(|byte| Ok(code_point + (((byte) & 0x3f) as u32)))?;
    Ok(code_point)
}

#[inline]
pub fn validate_next<I, U>(it: &mut I) -> Result<u32, UtfError>
where
    I: Iterator,
    <I as Iterator>::Item: AsByte<U>,
{
    let mut it = it.peekable();
    let lead = it.peek().map(|v| (*v).as_byte());
    let length = sequence_length(lead)?;
    let code_point = match length {
        SeqLen::One => get_sequence_1(&mut it),
        SeqLen::Two => get_sequence_2(&mut it),
        SeqLen::Three => get_sequence_3(&mut it),
        SeqLen::Four => get_sequence_4(&mut it),
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

pub trait AsByte<T>: Copy {
    fn as_byte(self) -> u8;
}

impl AsByte<u8> for u8 {
    fn as_byte(self) -> u8 {
        self
    }
}

impl AsByte<&u8> for &u8 {
    fn as_byte(self) -> u8 {
        *self
    }
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
    fn test_as_byte_by_value() {
        init_logger();
        let input = "qwerty";
        let mut it = input.bytes();
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
    fn test_as_byte_by_ref() {
        init_logger();
        let input = "qwerty";
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
        let input = "¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔ";
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
