use std::io::{Read, Write};

use crate::parse::{self, ArrayFlags, DataElement, DataElementTag, DataType};
use crate::Array;
use crate::NumericData;
use crate::{mat_error::MatError, MatFile};
use bytes::{BufMut, BytesMut};
use libflate::deflate::EncodeOptions;
use libflate::zlib::{Decoder, Encoder, Header};

const NAME_LENGTH_MAX: usize = 63;
const BEST_SPEED: u8 = 1;
pub fn write_header(mat: &MatFile) -> Result<BytesMut, MatError> {
    let mut header = BytesMut::with_capacity(128);
    let str_header = format!(
        "{}, Platform: PCWIN64, Created on: {}",
        mat.header.mat_identifier,
        chrono::Local::now().format("%a %b %e %T %Y").to_string()
    );
    header.put_slice(str_header.as_bytes());
    header.put_bytes(0x20, 116 - str_header.len());
    header.put_u64(mat.header.subsys_offset);
    header.put_u16(mat.header.version);
    header.put_slice(b"IM");
    Ok(header)
}
pub fn write_body(mat: &MatFile) -> Result<BytesMut, MatError> {
    let mut body_bytes = BytesMut::new();
    for arrays in mat.arrays.iter() {
        let data_element = write_next_data_element(arrays, mat.header.byte_order)?;
        body_bytes.put_slice(&data_element);
    }
    Ok(body_bytes)
}

// pub fn write_array_deflated(name:&str,array: &Array,)
fn write_next_data_element(
    array: &Array,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    //设置Compressed方式
    let data_element_bytes = write_matrix_data_element(array, endianness)?;
    println!("{:?}", data_element_bytes.to_vec());
    //加密矩阵的数组
    let compress_bytes = write_compressed_data_element(&data_element_bytes)?;
    let (data_element_tag_bytes, _) = write_data_element_tag(
        DataType::Compressed,
        compress_bytes.len() as u32,
        endianness,
    )?;
    bytes.put_slice(&data_element_tag_bytes);
    bytes.put_slice(&compress_bytes);
    Ok(bytes)
}
//zlib加密
fn write_compressed_data_element(source_bytes: &[u8]) -> Result<BytesMut, MatError> {
    let mut compressed_data_element = BytesMut::new();
    let mut encoder = Encoder::new(Vec::new()).unwrap();
    encoder.write_all(source_bytes)?;
    let buf = encoder.finish().into_result()?;
    compressed_data_element.put_slice(&buf);
    Ok(compressed_data_element)
}

//生成data_element_tag
////如果data_byte_size小于4采用Small Data Element Format
fn write_data_element_tag(
    data_type: DataType,
    data_byte_size: u32,
    endianness: nom::number::Endianness,
) -> Result<(BytesMut, u32), MatError> {
    let mut bytes = BytesMut::new();
    let packed = if data_byte_size < 4 { true } else { false };
    if !packed {
        // Long Data Element Format
        let data_type = data_type as u32;
        let padding = data_byte_size % 8;
        if endianness == nom::number::Endianness::Big {
            bytes.put_slice(&data_type.to_be_bytes());
            bytes.put_slice(&data_byte_size.to_be_bytes());
        } else {
            bytes.put_slice(&data_type.to_le_bytes());
            bytes.put_slice(&data_byte_size.to_le_bytes());
        }
        Ok((bytes, if padding == 0 { 0 } else { 8 - padding }))
    } else {
        // Small Data Element Format
        let data_type = data_type as u8;
        let data_byte_size = data_byte_size as u8;
        if endianness == nom::number::Endianness::Big {
            bytes.put_slice(&data_type.to_be_bytes());
            bytes.put_slice(&data_byte_size.to_be_bytes());
        } else {
            bytes.put_slice(&data_type.to_le_bytes());
            bytes.put_slice(&data_byte_size.to_le_bytes());
        }
        let padding = data_byte_size % 4;
        Ok((bytes, padding as u32))
    }
}
fn get_limited_name_size(name: &str) -> Result<usize, MatError> {
    let len = if name.is_empty() {
        0usize
    } else {
        NAME_LENGTH_MAX.min(name.as_bytes().len())
    };
    Ok(len)
}

fn write_matrix_data_element(
    array: &Array,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    let array_flags_bytes = write_array_flags_subelement(&array.array_flags, endianness)?;
    println!("flag={:?}", array_flags_bytes.to_vec());
    let subelements_bytes = write_numeric_matrix_subelements(array, endianness)?;
    println!("subelements_bytes={:?}", subelements_bytes.to_vec());
    let data_bytes_size = array_flags_bytes.len() + subelements_bytes.len();
    let (tag_bytes, padding) =
        write_data_element_tag(DataType::Matrix, data_bytes_size as u32, endianness)?;
    bytes.put_slice(&tag_bytes);
    bytes.put_slice(&array_flags_bytes);
    bytes.put_slice(&subelements_bytes);
    bytes.put_bytes(0, padding as usize);
    Ok(bytes)
}
fn write_array_flags_subelement(
    array_flags: &ArrayFlags,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    let tag_data_type = DataType::UInt32 as u32;
    let tag_data_len = 8u32;
    let mut flags_and_class = 0u32;
    if array_flags.complex {
        flags_and_class |= 0x0800;
    }
    if array_flags.global {
        flags_and_class |= 0x0400;
    }
    if array_flags.logical {
        flags_and_class |= 0x0200;
    }
    flags_and_class += array_flags.class as u32;
    let nzmax = array_flags.nzmax as u32;

    if endianness == nom::number::Endianness::Big {
        bytes.put_slice(&tag_data_type.to_be_bytes());
        bytes.put_slice(&tag_data_len.to_be_bytes());
        bytes.put_slice(&flags_and_class.to_be_bytes());
        bytes.put_slice(&nzmax.to_be_bytes());
    } else {
        bytes.put_slice(&tag_data_type.to_le_bytes());
        bytes.put_slice(&tag_data_len.to_le_bytes());
        bytes.put_slice(&flags_and_class.to_le_bytes());
        bytes.put_slice(&nzmax.to_le_bytes());
    }
    Ok(bytes)
}

//写入数字类型矩阵
fn write_numeric_matrix_subelements(
    array: &Array,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    //写入矩阵维数
    let dimensions_bytes = write_dimensions_array_subelement(array, endianness)?;
    bytes.put_slice(&dimensions_bytes);
    //写入矩阵名称
    let name_bytes = write_array_name_subelement(&array.name, endianness)?;
    bytes.put_slice(&name_bytes);
    //写入矩阵数据
    let numeric_bytes = write_numeric_subelement(array, endianness)?;
    bytes.put_slice(&numeric_bytes);
    Ok(bytes)
}
// pub fn is_packable(num_bytes: u32) -> bool {
//     if num_bytes < 4 {
//         true
//     } else {
//         false
//     }
// }
//写入矩阵维数
fn write_dimensions_array_subelement(
    array: &Array,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let data_type = array.array_flags.class.numeric_data_type().unwrap();
    let mut bytes = BytesMut::new();
    let dimension_size = array.size.len() * 4;
    let (dimension_tag_byte, padding) =
        write_data_element_tag(DataType::Int32, dimension_size as u32, endianness)?;
    bytes.put_slice(&dimension_tag_byte);
    for size in array.size.iter() {
        let dimension = *size as i32;
        if endianness == nom::number::Endianness::Big {
            bytes.put_i32(dimension);
        } else {
            bytes.put_i32_le(dimension);
        }
    }
    bytes.put_bytes(0, padding as usize);
    Ok(bytes)
}
//写入矩阵名称
fn write_array_name_subelement(
    name: &str,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    let name_size = get_limited_name_size(&name)?;
    let (name_tag_bytes, padding) =
        write_data_element_tag(DataType::Int8, name_size as u32, endianness)?;
    bytes.put_slice(&name_tag_bytes);
    bytes.put_slice(name.as_bytes());
    bytes.put_bytes(0, padding as usize);
    Ok(bytes)
}
//写入矩阵数据
fn write_numeric_subelement(
    array: &Array,
    endianness: nom::number::Endianness,
) -> Result<BytesMut, MatError> {
    let mut bytes = BytesMut::new();
    let (real_size, imag_size) = array.data.to_numberic_size();
    let (real_bytes, imag_bytes) = array.data.to_numberic_bytes(endianness);
    //处理实部
    let mat_type = if let Some(v) = array.array_flags.class.numeric_data_type() {
        v
    } else {
        return Err(MatError::ParamsError("该数据类型不存在".to_string()));
    };
    let (real_data_tag, padding) = write_data_element_tag(mat_type, real_size as u32, endianness)?;
    bytes.put_slice(&real_data_tag);
    bytes.put_slice(&real_bytes);
    bytes.put_bytes(0, padding as usize);
    //处理虚部
    if imag_size > 0 {
        let (imag_data_tag, padding) =
            write_data_element_tag(mat_type, imag_size as u32, endianness)?;
        bytes.put_slice(&imag_data_tag);
        bytes.put_slice(&imag_bytes);
        bytes.put_bytes(0, padding as usize);
    }
    Ok(bytes)
}

mod tests {
    use bytes::BufMut;
    use libflate::zlib::Decoder;
    use nom::character::complete::u32;
    use nom::sequence::tuple;
    use std::io::Read;

    #[test]
    fn write_array_flags_subelement() {
        let endianness = nom::number::Endianness::Little;
        let arraysflag = super::parse::ArrayFlags {
            complex: false,
            global: false,
            logical: false,
            class: super::parse::MatlabType::Opaque,
            nzmax: 0,
        };
        let r = super::write_array_flags_subelement(&arraysflag, endianness).unwrap();
        println!("r==>{:?}  {:?}", r, r.len());
        let r = super::parse::parse_array_flags_subelement(&r, endianness);
        println!("{:?}", r);
    }

    fn decoder(i: &[u8]) -> nom::IResult<&[u8], ()> {
        let mut buf = Vec::new();
        Decoder::new(i)
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
        println!("buf=={:?}", buf);
        Ok((&[], ()))
    }

    #[test]
    fn compress_test() {
        let bytes = b"Hello World!";
        let r = super::write_compressed_data_element(bytes).unwrap();
        println!("r=={:?}", r.to_vec());
        let r = decoder(&r);
        println!("{:?}", r);
    }
    #[test]
    fn vec_test() {
        println!("{}", 14 % 8);
    }
}
