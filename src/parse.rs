use bytes::BytesMut;
use libflate::zlib::Decoder;
use nom::branch::alt;
use nom::bytes::complete::take;
use nom::bytes::streaming::tag;
use nom::character::complete::alpha0;
use nom::combinator::map;
use nom::combinator::map_res;
use nom::combinator::peek;
use nom::combinator::value;
use nom::multi::count;
use nom::number::complete::{
    be_f32, be_f64, be_i16, be_i32, be_i64, be_i8, be_u16, be_u32, be_u64, be_u8, i32, le_f32,
    le_f64, le_i16, le_i32, le_i64, le_i8, le_u16, le_u32, le_u64, le_u8, u32,
};
use nom::number::Endianness;
use nom::sequence::tuple;
use nom::IResult;
use num_traits::FromPrimitive;
use std::io::Read;
#[derive(Debug, Clone)]
pub struct Header {
    pub version: u16,
    pub mat_identifier: String,
    pub description: String,
    pub byte_order: Endianness,
    pub subsys_offset: u64,
    pub deflate_level: i8, //NO_COMPRESSION:0,COMPRESSION:1
}

#[derive(Clone, Debug, PartialEq)]
pub enum NumericData {
    Int8(Vec<i8>),
    UInt8(Vec<u8>),
    Int16(Vec<i16>),
    UInt16(Vec<u16>),
    Int32(Vec<i32>),
    UInt32(Vec<u32>),
    Int64(Vec<i64>),
    UInt64(Vec<u64>),
    Single(Vec<f32>),
    Double(Vec<f64>),
}

impl NumericData {
    pub fn try_from(mat_type: MatlabType, rows: usize, cols: usize) -> Option<Self> {
        let nums = rows * cols;
        let data: Option<NumericData> = match mat_type {
            MatlabType::Int8 => {
                let mut v = Vec::new();
                v.resize(nums, 0_i8);
                Some(NumericData::Int8(v))
            }
            MatlabType::UInt8 => {
                let mut v = Vec::new();
                v.resize(nums, 0_u8);
                Some(NumericData::UInt8(v))
            }
            MatlabType::Int16 => {
                let mut v = Vec::new();
                v.resize(nums, 0_i16);
                Some(NumericData::Int16(v))
            }
            MatlabType::UInt16 => {
                let mut v = Vec::new();
                v.resize(nums, 0_u16);
                Some(NumericData::UInt16(v))
            }
            MatlabType::Int32 => {
                let mut v = Vec::new();
                v.resize(nums, 0_i32);
                Some(NumericData::Int32(v))
            }
            MatlabType::UInt32 => {
                let mut v = Vec::new();
                v.resize(nums, 0_u32);
                Some(NumericData::UInt32(v))
            }
            MatlabType::Int64 => {
                let mut v = Vec::new();
                v.resize(nums, 0_i64);
                Some(NumericData::Int64(v))
            }
            MatlabType::UInt64 => {
                let mut v = Vec::new();
                v.resize(nums, 0_u64);
                Some(NumericData::UInt64(v))
            }
            MatlabType::Single => {
                let mut v = Vec::new();
                v.resize(nums, 0_f32);
                Some(NumericData::Single(v))
            }
            MatlabType::Double => {
                let mut v = Vec::new();
                v.resize(nums, 0_f64);
                Some(NumericData::Double(v))
            }
            _ => None,
        };
        data
    }
    fn len(&self) -> usize {
        match self {
            NumericData::Single(vec) => vec.len(),
            NumericData::Double(vec) => vec.len(),
            NumericData::Int8(vec) => vec.len(),
            NumericData::UInt8(vec) => vec.len(),
            NumericData::Int16(vec) => vec.len(),
            NumericData::UInt16(vec) => vec.len(),
            NumericData::Int32(vec) => vec.len(),
            NumericData::UInt32(vec) => vec.len(),
            NumericData::Int64(vec) => vec.len(),
            NumericData::UInt64(vec) => vec.len(),
        }
    }

    fn data_type(&self) -> DataType {
        match self {
            NumericData::Single(_) => DataType::Single,
            NumericData::Double(_) => DataType::Double,
            NumericData::Int8(_) => DataType::Int8,
            NumericData::UInt8(_) => DataType::UInt8,
            NumericData::Int16(_) => DataType::Int16,
            NumericData::UInt16(_) => DataType::UInt16,
            NumericData::Int32(_) => DataType::Int32,
            NumericData::UInt32(_) => DataType::UInt32,
            NumericData::Int64(_) => DataType::Int64,
            NumericData::UInt64(_) => DataType::UInt64,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ArrayFlags {
    pub complex: bool,
    pub global: bool,
    pub logical: bool,
    pub class: MatlabType,
    pub nzmax: usize,
}

impl ArrayFlags {
    fn to_bytes(&self, endianness: nom::number::Endianness) -> BytesMut {
        let bytes = BytesMut::new();

        bytes
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Primitive)]
pub enum DataType {
    Int8 = 1,
    UInt8 = 2,
    Int16 = 3,
    UInt16 = 4,
    Int32 = 5,
    UInt32 = 6,
    Single = 7,
    Double = 9,
    Int64 = 12,
    UInt64 = 13,
    Matrix = 14,
    Compressed = 15,
    Utf8 = 16,
    Utf16 = 17,
    Utf32 = 18,
}

impl DataType {
    pub fn byte_size(&self) -> u32 {
        match self {
            DataType::Int8 | DataType::UInt8 | DataType::Utf8 => 1,
            DataType::Int16 | DataType::UInt16 | DataType::Utf16 => 2,
            DataType::Int32 | DataType::UInt32 | DataType::Single | DataType::Utf32 => 4,
            DataType::Int64 | DataType::UInt64 | DataType::Double => 8,
            _ => 1,
        }
    }
    pub fn get_padding(&self, num_bytes: u32, packed: bool) -> u32 {
        match self {
            DataType::Matrix | DataType::Compressed => 0,
            _ => {
                let tag_size = if packed { 4 } else { 8 };
                let padding = (tag_size + num_bytes) % 8;
                if padding == 0 {
                    0
                } else {
                    8 - padding
                }
            }
        }
    }
    pub fn get_numbytes(&self, num_elements: u32) -> u32 {
        self.byte_size() * num_elements
    }
    pub fn is_packable(&self, num_bytes: u32) -> bool {
        if num_bytes < 4 {
            true
        } else {
            false
        }
    }
    pub fn computer_array_size(&self, num_elements: u32) -> u32 {
        let num_bytes = self.get_numbytes(num_elements);
        let packed = self.is_packable(num_elements);
        let tag_size = if packed { 4 } else { 8 };
        let padding = self.get_padding(num_bytes, packed);
        tag_size + num_bytes + padding
    }
    // fn write_padding(&self, num_elements: u32) -> BytesMut {
    //     let mut bytes = BytesMut::new();
    //     let num_bytes = self.get_numbytes(num_elements);
    //     let padding = self.get_padding(num_bytes, self.is_packabl(num_bytes));
    //     if padding == 0 {
    //         bytes
    //     } else {
    //         bytes
    //     }
    // }
}

#[derive(Debug, PartialEq, Clone, Copy, Primitive)]
pub enum MatlabType {
    Cell = 1,
    Struct = 2,
    Object = 3,
    Char = 4,
    Sparse = 5,
    Double = 6,
    Single = 7,
    Int8 = 8,
    UInt8 = 9,
    Int16 = 10,
    UInt16 = 11,
    Int32 = 12,
    UInt32 = 13,
    Int64 = 14,
    UInt64 = 15,
    Function = 16,
    Opaque = 17,
}
impl MatlabType {
    pub fn numeric_data_type(&self) -> Option<DataType> {
        match self {
            MatlabType::Double => Some(DataType::Double),
            MatlabType::Single => Some(DataType::Single),
            MatlabType::Int8 => Some(DataType::Int8),
            MatlabType::UInt8 => Some(DataType::UInt8),
            MatlabType::Int16 => Some(DataType::Int16),
            MatlabType::UInt16 => Some(DataType::UInt16),
            MatlabType::Int32 => Some(DataType::UInt32),
            MatlabType::UInt32 => Some(DataType::UInt32),
            MatlabType::Int64 => Some(DataType::Int64),
            MatlabType::UInt64 => Some(DataType::UInt64),
            _ => None,
        }
    }
}
pub type Dimensions = Vec<i32>;
pub type RowIndex = Vec<usize>;
pub type ColumnShift = Vec<usize>;

#[derive(Clone, Debug)]
pub enum DataElement {
    NumericMatrix(
        ArrayFlags,
        Dimensions,
        String,
        NumericData,
        Option<NumericData>,
    ),
    SparseMatrix(
        ArrayFlags,
        Dimensions,
        String,
        RowIndex,
        ColumnShift,
        NumericData,
        Option<NumericData>,
    ),
    // CharacterMatrix,
    // Cell Matrix,
    // Structure Matrix,
    // Object Matrix,
    Unsupported,
}
#[derive(Clone, Debug)]
pub struct DataElementTag {
    pub data_type: DataType,
    pub data_byte_size: u32,
    pub padding_byte_size: u32,
}
pub struct ParseResult {
    pub header: Header,
    pub data_elements: Vec<DataElement>,
}

pub fn parse_header(i: &[u8]) -> IResult<&[u8], Header> {
    let text = take(116usize)(i)?;
    let _ssdo = take(8usize)(text.0)?;
    let (input, (text, ssdo, version)) = tuple((take(116usize), take(8usize), take(2usize)))(i)?;
    let (input, is_little_endian) = alt((value(true, tag("IM")), value(false, tag("MI"))))(input)?;
    let mut v_ssdo = ssdo.to_vec();
    v_ssdo.reverse();
    let sub_sys_offset = v_ssdo.iter().fold(0, |x, &i| x << 8 | i as u64);
    let version = version.iter().fold(0, |x, &i| x << 8 | i as u64);
    Ok((
        input,
        Header {
            description: std::str::from_utf8(text).unwrap_or(&"").to_owned(),
            byte_order: if is_little_endian {
                Endianness::Little
            } else {
                Endianness::Big
            },
            subsys_offset: sub_sys_offset,
            version: version as u16,
            deflate_level: 1,
            mat_identifier: "".to_lowercase(),
        },
    ))
}
fn ceil_to_multiple(x: u32, multiple: u32) -> u32 {
    if x > 0 {
        (((x - 1) / multiple) + 1) * multiple
    } else {
        0
    }
}
fn get_padding(data_type: u32, byte_size: u32, packed: bool) -> u32 {
    if data_type == 14 || data_type == 15 {
        0
    } else {
        let tag_size = if packed == true { 4 } else { 8 };
        let padding = (tag_size + byte_size) % 8;
        if padding == 0 {
            0
        } else {
            8 - padding
        }
    }
}
fn parse_data_element_tag(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], DataElementTag> {
    let (input, flag) = map(peek(u32(endianness)), |b| b & 0xFFFF0000)(i)?;
    match flag {
        0 => {
            //Long Data Format
            let (input, (data_type, byte_size)) = tuple((u32(endianness), u32(endianness)))(input)?;
            let padding_size = get_padding(data_type, byte_size, false);
            println!(
                "Tag0 data_type={:?} byte_size={:?} padding={:?}",
                data_type, byte_size, padding_size
            );
            Ok((
                input,
                DataElementTag {
                    data_type: DataType::from_u32(data_type).ok_or(nom::Err::Failure(
                        nom::error::Error {
                            input: i,
                            code: nom::error::ErrorKind::Tag,
                        },
                    ))?,
                    data_byte_size: byte_size,
                    padding_byte_size: padding_size,
                },
            ))
        }
        _ => {
            //Small Data Format
            let (input, data_type) = map(peek(u32(endianness)), |b| b & 0x0000FFFF)(input)?;
            let (input, byte_size) = map(u32(endianness), |b| (b & 0xFFFF0000) >> 16)(input)?;
            println!(
                "Tag1 data_type={:?} byte_size={:?} padding={:?}",
                data_type,
                byte_size,
                4 - byte_size as u32
            );
            Ok((
                input,
                DataElementTag {
                    data_type: DataType::from_u32(data_type).ok_or(nom::Err::Failure(
                        nom::error::Error {
                            input: i,
                            code: nom::error::ErrorKind::Tag,
                        },
                    ))?,
                    // TODO: assert that byte_size is <= 4
                    data_byte_size: byte_size as u32,
                    padding_byte_size: 4 - byte_size as u32,
                },
            ))
        }
    }
}

pub fn replace_err_slice<'old, 'new>(
    err: nom::Err<nom::error::Error<&'old [u8]>>,
    new_slice: &'new [u8],
) -> nom::Err<nom::error::Error<&'new [u8]>> {
    match err {
        nom::Err::Error(nom::error::Error { code, .. }) => nom::Err::Error(nom::error::Error {
            code,
            input: new_slice,
        }),
        nom::Err::Failure(nom::error::Error { code, .. }) => nom::Err::Failure(nom::error::Error {
            code,
            input: new_slice,
        }),
        nom::Err::Incomplete(needed) => nom::Err::Incomplete(needed),
    }
}

fn assert(i: &[u8], v: bool) -> IResult<&[u8], ()> {
    if v {
        Ok((i, ()))
    } else {
        Err(nom::Err::Failure(error_position!(
            i,
            // TODO
            nom::error::ErrorKind::Tag
        )))
    }
}
pub fn parse_array_flags_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], ArrayFlags> {
    let (input, (tag_data_type, tag_data_len, flags_and_class, nzmax)) = tuple((
        u32(endianness),
        u32(endianness),
        u32(endianness),
        u32(endianness),
    ))(i)?;
    println!(
        "arrayflags==>tag_data_type={} tag_data_len={} flags_and_class={} nzmax={} ",
        tag_data_type, tag_data_len, flags_and_class, nzmax
    );
    assert(
        input,
        tag_data_type == DataType::UInt32 as u32 && tag_data_len == 8,
    )?;
    Ok((
        input,
        ArrayFlags {
            complex: (flags_and_class & 0x0800) != 0,
            global: (flags_and_class & 0x0400) != 0,
            logical: (flags_and_class & 0x0200) != 0,
            class: MatlabType::from_u8((flags_and_class & 0xFF) as u8).ok_or(
                nom::Err::Failure(nom::error::Error {
                    input: i,
                    code: nom::error::ErrorKind::Tag,
                }), // TODO
            )?,
            nzmax: nzmax as usize,
        },
    ))
}
fn parse_row_index_array_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], RowIndex> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    let (input, row_index) = count(
        i32(endianness),
        (data_element_tag.data_byte_size / 4) as usize,
    )(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    let rows = row_index.iter().map(|&i| i as usize).collect();
    Ok((input, rows))
}
fn parse_column_index_array_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], ColumnShift> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    let (input, column_index) = count(
        i32(endianness),
        (data_element_tag.data_byte_size / 4) as usize,
    )(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    let columns = column_index.iter().map(|&i| i as usize).collect();
    Ok((input, columns))
}

fn parse_sparse_matrix_subelements(
    i: &[u8],
    endianness: nom::number::Endianness,
    flags: ArrayFlags,
) -> IResult<&[u8], DataElement> {
    let (input, dimensions) = parse_dimensions_array_subelement(i, endianness)?;
    let (input, name) = parse_array_name_subelement(input, endianness)?;
    let (input, row_index) = parse_row_index_array_subelement(input, endianness)?;
    let (input, column_index) = parse_column_index_array_subelement(input, endianness)?;
    //先暂时不要插入检查语句
    let (input, real_part) = parse_numeric_subelement(input, endianness)?;
    let (input, imag_part) = if flags.complex {
        let (input, imag_part) = parse_numeric_subelement(input, endianness)?;
        (input, Some(imag_part))
    } else {
        (input, None)
    };
    Ok((
        input,
        DataElement::SparseMatrix(
            flags,
            dimensions,
            name,
            row_index.iter().map(|&i| i as usize).collect(),
            column_index.iter().map(|&i| i as usize).collect(),
            real_part,
            imag_part,
        ),
    ))
}
fn parse_unsupported_data_element(
    _i: &[u8],
    _endianness: nom::number::Endianness,
) -> IResult<&[u8], DataElement> {
    Ok((&[], DataElement::Unsupported))
}
fn parse_matrix_data_element(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], DataElement> {
    let (input, flags) = parse_array_flags_subelement(i, endianness)?;
    println!("arrayflags={:?}  class={:?}", flags, flags.class);
    let (input, data_element) = match flags.class {
        MatlabType::Cell => parse_unsupported_data_element(input, endianness)?,
        MatlabType::Struct => parse_unsupported_data_element(input, endianness)?,
        MatlabType::Object => parse_unsupported_data_element(input, endianness)?,
        MatlabType::Char => parse_unsupported_data_element(input, endianness)?,
        MatlabType::Sparse => parse_sparse_matrix_subelements(input, endianness, flags)?,
        MatlabType::Opaque => parse_opaque_matrix_subelements(input, endianness)?,
        _ => parse_numeric_matrix_subelements(input, endianness, flags)?,
    };
    Ok((input, data_element))
}
fn parse_opaque_matrix_subelements(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], DataElement> {
    println!("parse_opaque");
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    let (input, name) = take(data_element_tag.data_byte_size)(input)?;
    println!("name={:?}", String::from_utf8_lossy(name));
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;

    let (input, data_element_tag) = parse_data_element_tag(input, endianness)?;
    let (input, object_type) = take(data_element_tag.data_byte_size)(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    println!(
        "objectType={:?} {:?}",
        String::from_utf8_lossy(object_type),
        data_element_tag.data_byte_size
    );
    let (input, data_element_tag) = parse_data_element_tag(input, endianness)?;
    let (input, class_name) = take(data_element_tag.data_byte_size)(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    println!(
        "className={:?} {:?}",
        String::from_utf8_lossy(class_name),
        data_element_tag.data_byte_size
    );
    let (input, dimensions) = parse_dimensions_array_subelement(input, endianness)?;
    println!("dimensions==>{:?}", dimensions);
    Ok((input, DataElement::Unsupported))
}

pub fn parse_compressed_data_element(
    i: &[u8],
    endianness: nom::number::Endianness,
    byte_size: u32,
) -> IResult<&[u8], DataElement> {
    let mut buf = Vec::new();
    let (input, compress_data) = take(byte_size)(i)?;
    Decoder::new(compress_data)
        .map_err(|err| {
            eprintln!("{:?}", err);
            nom::Err::Failure(nom::error::Error {
                input: i,
                code: nom::error::ErrorKind::Tag,
            }) // TODO
        })?
        .read_to_end(&mut buf)
        .map_err(|err| {
            eprintln!("{:?}", err);
            nom::Err::Failure(nom::error::Error {
                input: i,
                code: nom::error::ErrorKind::Tag,
            }) // TODO
        })?;

    println!("buf==>{:?}", buf);
    let (_remaining, data_element) = parse_next_data_element(buf.as_slice(), endianness)
        .map_err(|err| replace_err_slice(err, i))?;
    // Ok((&[], data_element))
    Ok((input, data_element))
}
fn parse_dimensions_array_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], Dimensions> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    let (input, dimensions) = count(
        i32(endianness),
        (data_element_tag.data_byte_size / 4) as usize,
    )(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    Ok((input, dimensions))
}
fn parse_array_name_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], String> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    let (input, name) = map_res(take(data_element_tag.data_byte_size), |b| {
        std::str::from_utf8(b)
            .map(|s| s.to_owned())
            .map_err(|_err| {
                nom::Err::Failure((i, nom::error::ErrorKind::Tag)) // TODO
            })
    })(input)?;
    let (input, _) = take(data_element_tag.padding_byte_size)(input)?;
    println!("name===>{:?}", name);
    Ok((input, name))
}

fn parse_numeric_subelement(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], NumericData> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    println!("数据类型={:?}", data_element_tag);
    let (input, numeric_data) = match data_element_tag.data_type {
        DataType::Int8 => {
            if endianness == Endianness::Big {
                let (input, data) = count(be_i8, data_element_tag.data_byte_size as usize)(input)?;
                (input, NumericData::Int8(data))
            } else {
                let (input, data) = count(le_i8, data_element_tag.data_byte_size as usize)(input)?;
                (input, NumericData::Int8(data))
            }
        }
        DataType::UInt8 => {
            if endianness == Endianness::Big {
                let (input, data) = count(be_u8, data_element_tag.data_byte_size as usize)(input)?;
                (input, NumericData::UInt8(data))
            } else {
                let (input, data) = count(le_u8, data_element_tag.data_byte_size as usize)(input)?;
                (input, NumericData::UInt8(data))
            }
        }
        DataType::Int16 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_i16, data_element_tag.data_byte_size as usize / 2)(input)?;
                (input, NumericData::Int16(data))
            } else {
                let (input, data) =
                    count(le_i16, data_element_tag.data_byte_size as usize / 2)(input)?;
                (input, NumericData::Int16(data))
            }
        }
        DataType::UInt16 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_u16, data_element_tag.data_byte_size as usize / 2)(input)?;
                (input, NumericData::UInt16(data))
            } else {
                let (input, data) =
                    count(le_u16, data_element_tag.data_byte_size as usize / 2)(input)?;
                (input, NumericData::UInt16(data))
            }
        }
        DataType::Int32 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_i32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::Int32(data))
            } else {
                let (input, data) =
                    count(le_i32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::Int32(data))
            }
        }
        DataType::UInt32 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_u32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::UInt32(data))
            } else {
                let (input, data) =
                    count(le_u32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::UInt32(data))
            }
        }
        DataType::Int64 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_i64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::Int64(data))
            } else {
                let (input, data) =
                    count(le_i64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::Int64(data))
            }
        }
        DataType::UInt64 => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_u64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::UInt64(data))
            } else {
                let (input, data) =
                    count(le_u64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::UInt64(data))
            }
        }
        DataType::Single => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_f32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::Single(data))
            } else {
                let (input, data) =
                    count(le_f32, data_element_tag.data_byte_size as usize / 4)(input)?;
                (input, NumericData::Single(data))
            }
        }
        DataType::Double => {
            if endianness == Endianness::Big {
                let (input, data) =
                    count(be_f64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::Double(data))
            } else {
                let (input, data) =
                    count(le_f64, data_element_tag.data_byte_size as usize / 8)(input)?;
                (input, NumericData::Double(data))
            }
        }
        _ => {
            return Err(nom::Err::Failure(error_position!(
                i,
                // TODO
                nom::error::ErrorKind::Tag
            )));
        }
    };
    take(data_element_tag.padding_byte_size)(input)?;
    Ok((input, numeric_data))
}

fn parse_numeric_matrix_subelements(
    i: &[u8],
    endianness: nom::number::Endianness,
    flags: ArrayFlags,
) -> IResult<&[u8], DataElement> {
    println!("dimensions");
    let (input, dimensions) = parse_dimensions_array_subelement(i, endianness)?;
    let (input, name) = parse_array_name_subelement(input, endianness)?;
    let (input, real_part) = parse_numeric_subelement(input, endianness)?;
    // let (input, n_required_elements) = value(dimensions.iter().product::<i32>(), alpha0)(input)?;
    // let (input, array_data_type) = value(flags.class.numeric_data_type().unwrap(), alpha0)(input)?;
    let (input, imag_part) = if flags.complex {
        let (input, imag_part) = parse_numeric_subelement(input, endianness)?;
        (input, Some(imag_part))
    } else {
        (input, None)
    };
    Ok((
        input,
        DataElement::NumericMatrix(flags, dimensions, name, real_part, imag_part),
    ))
}

fn parse_next_data_element(
    i: &[u8],
    endianness: nom::number::Endianness,
) -> IResult<&[u8], DataElement> {
    let (input, data_element_tag) = parse_data_element_tag(i, endianness)?;
    println!("data_element_tag=={:?}", data_element_tag);
    let (input, data_element) = match data_element_tag.data_type {
        DataType::Matrix => parse_matrix_data_element(input, endianness)?,
        DataType::Compressed => {
            parse_compressed_data_element(input, endianness, data_element_tag.data_byte_size)?
        }
        _ => parse_unsupported_data_element(input, endianness)?,
    };
    Ok((input, data_element))
    // let (input, data) = take(data_element_tag.data_byte_size)(input)?;
    // Ok((input, DataElement::Unsupported))
}

pub fn parse_all(i: &[u8]) -> IResult<&[u8], ParseResult> {
    let (mut input, header) = parse_header(i)?;
    println!("{:?}", header);
    let mut data_elements = vec![];
    loop {
        match parse_next_data_element(input, header.byte_order) {
            Ok((new_input, data_element)) => {
                input = new_input;
                data_elements.push(data_element);
            }
            _ => {
                break;
            }
        };
    }
    // println!("matfile==>{:?}", data_elements);
    Ok((
        input,
        ParseResult {
            header: header,
            data_elements: data_elements,
        },
    ))
}

mod tests {
    use crate::parse::le_f64;
    use crate::parse::DataElement;
    use nom::multi::count;
    use nom::IResult;

    #[test]
    fn name() {
        let data = include_bytes!("d:/myfile.mat");
        let r = super::parse_all(data).unwrap();
    }
    #[test]
    fn test_product() {
        let v = vec![1, 2, -3];
        let s = v.iter().product::<i32>();
        println!("{}", s);
    }

    #[test]
    fn decode() {
        let encoded_data = [
            120, 156, 243, 72, 205, 201, 201, 87, 8, 207, 47, 202, 73, 81, 4, 0, 28, 73, 4, 62,
        ];
        let decoder = libflate::zlib::Decoder::new(&encoded_data[..]).unwrap();
        println!("decode=={:?}", decoder);
    }
    fn test_count(i: &[u8]) -> IResult<&[u8], DataElement> {
        let array = &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x29, 0x40][..];
        println!("{:?}", array);
        let result = count(le_f64, 1)(array)?;
        println!("{:?}", result);
        Ok((&[], DataElement::Unsupported))
    }
}
