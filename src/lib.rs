#[macro_use]
extern crate nom;
#[macro_use]
extern crate enum_primitive_derive;
extern crate log;
mod mat_error;
#[cfg(feature = "ndarray")]
pub mod ndarray;
mod parse;
mod writer;

use std::io::Write;

use bytes::{BufMut, BytesMut};
use nom::number::Endianness;
use parse::Header;

use crate::mat_error::MatError;

#[derive(Clone, Debug)]
pub struct Array {
    array_flags: parse::ArrayFlags,
    name: String,
    size: Vec<usize>,
    data: NumericData,
}
impl Array {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn size(&self) -> &Vec<usize> {
        &self.size
    }
    pub fn ndims(&self) -> usize {
        self.size.len()
    }
    pub fn data(&self) -> &NumericData {
        &self.data
    }
    pub fn get_num_elements(&self) -> u32 {
        let mut count = 1;
        for val in self.size.iter() {
            count *= val;
        }
        count as u32
    }
    pub fn check_index_bound(&self, index: usize, dim: usize) -> usize {
        if index >= 0 && index < self.size[dim] {
            index
        } else {
            dim
        }
    }
    pub fn set_int8(&mut self, row: usize, col: usize, value: i8) {
        let ix0 = self.check_index_bound(row, 0);
        let ix1 = self.check_index_bound(col, 1);
        let index = row + col * self.size[0];
        // println!("index={} x={} y={} value={}", index, ix0, ix1, value);
        match &mut self.data {
            NumericData::Int8 { real, .. } => {
                real[index] = value;
            }
            _ => {}
        }
    }

    // pub fn set_imaginary_int8(&mut self, value: Vec<i8>) {}
}

#[derive(Clone, Debug)]
pub enum NumericData {
    Int8 {
        real: Vec<i8>,
        imag: Option<Vec<i8>>,
    },
    UInt8 {
        real: Vec<u8>,
        imag: Option<Vec<u8>>,
    },
    Int16 {
        real: Vec<i16>,
        imag: Option<Vec<i16>>,
    },
    UInt16 {
        real: Vec<u16>,
        imag: Option<Vec<u16>>,
    },
    Int32 {
        real: Vec<i32>,
        imag: Option<Vec<i32>>,
    },
    UInt32 {
        real: Vec<u32>,
        imag: Option<Vec<u32>>,
    },
    Int64 {
        real: Vec<i64>,
        imag: Option<Vec<i64>>,
    },
    UInt64 {
        real: Vec<u64>,
        imag: Option<Vec<u64>>,
    },
    Single {
        real: Vec<f32>,
        imag: Option<Vec<f32>>,
    },
    Double {
        real: Vec<f64>,
        imag: Option<Vec<f64>>,
    },
}
impl NumericData {
    fn to_numberic_bytes(&self, endianness: nom::number::Endianness) -> (BytesMut, BytesMut) {
        let mut real_bytes = BytesMut::new();
        let mut imag_bytes = BytesMut::new();
        match self {
            NumericData::Int8 { real, imag } => {
                for v in real {
                    real_bytes.put_i8(v.to_owned());
                }
            }
            NumericData::UInt8 { real, imag } => {
                for v in real {
                    real_bytes.put_u8(v.to_owned());
                }
            }
            NumericData::Int16 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_i16(v.to_owned());
                    } else {
                        real_bytes.put_i16_le(v.to_owned());
                    }
                }
            }
            NumericData::UInt16 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_u16(v.to_owned());
                    } else {
                        real_bytes.put_u16_le(v.to_owned());
                    }
                }
            }
            NumericData::Int32 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_i32(v.to_owned());
                    } else {
                        real_bytes.put_i32_le(v.to_owned());
                    }
                }
            }
            NumericData::UInt32 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_u32(v.to_owned());
                    } else {
                        real_bytes.put_u32_le(v.to_owned());
                    }
                }
            }
            NumericData::Int64 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_i64(v.to_owned());
                    } else {
                        real_bytes.put_i64_le(v.to_owned());
                    }
                }
            }
            NumericData::UInt64 { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_u64(v.to_owned());
                    } else {
                        real_bytes.put_u64_le(v.to_owned());
                    }
                }
            }
            NumericData::Single { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_f32(v.to_owned());
                    } else {
                        real_bytes.put_f32_le(v.to_owned());
                    }
                }
            }
            NumericData::Double { real, imag } => {
                for v in real {
                    if endianness == nom::number::Endianness::Big {
                        real_bytes.put_f64(v.to_owned());
                    } else {
                        real_bytes.put_f64_le(v.to_owned());
                    }
                }
            }
            _ => {}
        };
        (real_bytes, imag_bytes)
    }
    fn to_numberic_size(&self) -> (usize, usize) {
        match self {
            NumericData::Int8 { real, imag } => {
                (real.len(), if let Some(v) = imag { v.len() } else { 0 })
            }
            NumericData::UInt8 { real, imag } => {
                (real.len(), if let Some(v) = imag { v.len() } else { 0 })
            }
            NumericData::Int16 { real, imag } => (
                2 * real.len(),
                if let Some(v) = imag { v.len() * 2 } else { 0 },
            ),
            NumericData::UInt16 { real, imag } => (
                2 * real.len(),
                if let Some(v) = imag { v.len() * 2 } else { 0 },
            ),
            NumericData::Int32 { real, imag } => (
                4 * real.len(),
                if let Some(v) = imag { v.len() * 4 } else { 0 },
            ),
            NumericData::UInt32 { real, imag } => (
                4 * real.len(),
                if let Some(v) = imag { v.len() * 4 } else { 0 },
            ),
            NumericData::Int64 { real, imag } => (
                8 * real.len(),
                if let Some(v) = imag { v.len() * 8 } else { 0 },
            ),
            NumericData::UInt64 { real, imag } => (
                8 * real.len(),
                if let Some(v) = imag { v.len() * 8 } else { 0 },
            ),
            NumericData::Single { real, imag } => (
                4 * real.len(),
                if let Some(v) = imag { v.len() * 4 } else { 0 },
            ),
            NumericData::Double { real, imag } => (
                8 * real.len(),
                if let Some(v) = imag { v.len() * 8 } else { 0 },
            ),
            _ => (0usize, 0usize),
        }
    }
    fn to_real_size(&self) -> usize {
        match self {
            NumericData::Int8 { real, imag } => real.len(),
            NumericData::UInt8 { real, imag } => real.len(),
            NumericData::Int16 { real, imag } => 2 * real.len(),
            NumericData::UInt16 { real, imag } => 2 * real.len(),
            NumericData::Int32 { real, imag } => 4 * real.len(),
            NumericData::UInt32 { real, imag } => 4 * real.len(),
            NumericData::Int64 { real, imag } => 8 * real.len(),
            NumericData::UInt64 { real, imag } => 8 * real.len(),
            NumericData::Single { real, imag } => 4 * real.len(),
            NumericData::Double { real, imag } => 8 * real.len(),
            _ => 0,
        }
    }
}

fn try_convert_number_format(
    target_type: parse::MatlabType,
    data: parse::NumericData,
) -> Result<parse::NumericData, MatError> {
    match target_type {
        parse::MatlabType::Double => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::Double(
                data.into_iter().map(|x| x as f64).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::Double(
                data.into_iter().map(|x| x as f64).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::Double(
                data.into_iter().map(|x| x as f64).collect(),
            )),
            parse::NumericData::Int32(data) => Ok(parse::NumericData::Double(
                data.into_iter().map(|x| x as f64).collect(),
            )),
            parse::NumericData::Double(data) => Ok(parse::NumericData::Double(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::Single => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::Single(
                data.into_iter().map(|x| x as f32).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::Single(
                data.into_iter().map(|x| x as f32).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::Single(
                data.into_iter().map(|x| x as f32).collect(),
            )),
            parse::NumericData::Int32(data) => Ok(parse::NumericData::Single(
                data.into_iter().map(|x| x as f32).collect(),
            )),
            parse::NumericData::Single(data) => Ok(parse::NumericData::Single(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::UInt64 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::UInt64(
                data.into_iter().map(|x| x as u64).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::UInt64(
                data.into_iter().map(|x| x as u64).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::UInt64(
                data.into_iter().map(|x| x as u64).collect(),
            )),
            parse::NumericData::Int32(data) => Ok(parse::NumericData::UInt64(
                data.into_iter().map(|x| x as u64).collect(),
            )),
            parse::NumericData::UInt64(data) => Ok(parse::NumericData::UInt64(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::Int64 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::Int64(
                data.into_iter().map(|x| x as i64).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::Int64(
                data.into_iter().map(|x| x as i64).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::Int64(
                data.into_iter().map(|x| x as i64).collect(),
            )),
            parse::NumericData::Int32(data) => Ok(parse::NumericData::Int64(
                data.into_iter().map(|x| x as i64).collect(),
            )),
            parse::NumericData::Int64(data) => Ok(parse::NumericData::Int64(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::UInt32 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::UInt32(
                data.into_iter().map(|x| x as u32).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::UInt32(
                data.into_iter().map(|x| x as u32).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::UInt32(
                data.into_iter().map(|x| x as u32).collect(),
            )),
            parse::NumericData::UInt32(data) => Ok(parse::NumericData::UInt32(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::Int32 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::Int32(
                data.into_iter().map(|x| x as i32).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::Int32(
                data.into_iter().map(|x| x as i32).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::Int32(
                data.into_iter().map(|x| x as i32).collect(),
            )),
            parse::NumericData::Int32(data) => Ok(parse::NumericData::Int32(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::UInt16 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::UInt16(
                data.into_iter().map(|x| x as u16).collect(),
            )),
            parse::NumericData::UInt16(data) => Ok(parse::NumericData::UInt16(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::Int16 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::Int16(
                data.into_iter().map(|x| x as i16).collect(),
            )),
            parse::NumericData::Int16(data) => Ok(parse::NumericData::Int16(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::UInt8 => match data {
            parse::NumericData::UInt8(data) => Ok(parse::NumericData::UInt8(data)),
            _ => Err(MatError::ConversionError),
        },
        parse::MatlabType::Int8 => match data {
            parse::NumericData::Int8(data) => Ok(parse::NumericData::Int8(data)),
            _ => Err(MatError::ConversionError),
        },
        _ => Err(MatError::ConversionError),
    }
}

impl NumericData {
    fn try_from(
        target_type: parse::MatlabType,
        real: parse::NumericData,
        imag: Option<parse::NumericData>,
    ) -> Result<Self, MatError> {
        let real = try_convert_number_format(target_type, real)?;
        let imag = match imag {
            Some(imag) => Some(try_convert_number_format(target_type, imag)?),
            None => None,
        };
        match (real, imag) {
            (parse::NumericData::Double(real), None) => Ok(NumericData::Double {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Double(real), Some(parse::NumericData::Double(imag))) => {
                Ok(NumericData::Double {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::Single(real), None) => Ok(NumericData::Single {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Single(real), Some(parse::NumericData::Single(imag))) => {
                Ok(NumericData::Single {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::UInt64(real), None) => Ok(NumericData::UInt64 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::UInt64(real), Some(parse::NumericData::UInt64(imag))) => {
                Ok(NumericData::UInt64 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::Int64(real), None) => Ok(NumericData::Int64 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Int64(real), Some(parse::NumericData::Int64(imag))) => {
                Ok(NumericData::Int64 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::UInt32(real), None) => Ok(NumericData::UInt32 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::UInt32(real), Some(parse::NumericData::UInt32(imag))) => {
                Ok(NumericData::UInt32 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::Int32(real), None) => Ok(NumericData::Int32 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Int32(real), Some(parse::NumericData::Int32(imag))) => {
                Ok(NumericData::Int32 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::UInt16(real), None) => Ok(NumericData::UInt16 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::UInt16(real), Some(parse::NumericData::UInt16(imag))) => {
                Ok(NumericData::UInt16 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::Int16(real), None) => Ok(NumericData::Int16 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Int16(real), Some(parse::NumericData::Int16(imag))) => {
                Ok(NumericData::Int16 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::UInt8(real), None) => Ok(NumericData::UInt8 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::UInt8(real), Some(parse::NumericData::UInt8(imag))) => {
                Ok(NumericData::UInt8 {
                    real: real,
                    imag: Some(imag),
                })
            }
            (parse::NumericData::Int8(real), None) => Ok(NumericData::Int8 {
                real: real,
                imag: None,
            }),
            (parse::NumericData::Int8(real), Some(parse::NumericData::Int8(imag))) => {
                Ok(NumericData::Int8 {
                    real: real,
                    imag: Some(imag),
                })
            }
            _ => return Err(MatError::InternalError),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatFile {
    header: Header,
    arrays: Vec<Array>,
}
impl MatFile {
    pub fn add_array(&mut self, array: Array) -> &mut MatFile {
        self.arrays.push(array);
        self
    }
    pub fn find_by_name<'a>(&'a self, name: &'_ str) -> Option<&'a Array> {
        for array in &self.arrays {
            if array.name == name {
                return Some(array);
            }
        }
        None
    }

    pub fn new_mat_file() -> MatFile {
        // 检查CPU大端和小端模式
        let v: u16 = 0x00ff;
        let first_octet: u8 = unsafe {
            let ptr = &v as *const u16;
            let ptr = ptr as *const u8;
            *ptr
        };
        MatFile {
            arrays: vec![],
            header: Header {
                version: 1,
                mat_identifier: "MATLAB 5.0 MAT-file".to_string(),
                description: "".to_string(),
                byte_order: if first_octet == 0xff {
                    Endianness::Little
                } else {
                    Endianness::Big
                },
                subsys_offset: 0,
                deflate_level: 1,
            },
        }
    }
    pub fn new_matrix(
        name: &str,
        rows: usize,
        cols: usize,
        complex: bool,
        mat_type: parse::MatlabType,
    ) -> Result<Array, MatError> {
        let array_flags = parse::ArrayFlags {
            complex: complex,
            global: false,
            logical: false,
            class: mat_type,
            nzmax: 0,
        };
        let real = if let Some(data) = parse::NumericData::try_from(mat_type, rows, cols) {
            data
        } else {
            return Err(MatError::ParamsError("暂时不支持该数据类型".to_string()));
        };
        let imag = if complex {
            parse::NumericData::try_from(mat_type, rows, cols)
        } else {
            None
        };
        let data = NumericData::try_from(mat_type, real, imag)?;
        let array = Array {
            array_flags: array_flags,
            name: name.to_string(),
            size: vec![rows, cols],
            data: data,
        };
        Ok(array)
    }
    pub fn parse<R: std::io::Read>(mut read: R) -> Result<Self, MatError> {
        let mut buf = Vec::new();
        read.read_to_end(&mut buf)
            .map_err(|err| MatError::IOError(err))?;
        let (_remaining, parse_result) = parse::parse_all(&buf)
            .map_err(|err| MatError::ParseError(parse::replace_err_slice(err, &[])))?;
        let arrays: Result<Vec<Array>, MatError> = parse_result
            .data_elements
            .into_iter()
            .filter_map(|data_element| match data_element {
                parse::DataElement::NumericMatrix(flags, dims, name, real, imag) => {
                    let size = dims.into_iter().map(|d| d as usize).collect();
                    let numeric_data = match NumericData::try_from(flags.class, real, imag) {
                        Ok(numeric_data) => numeric_data,
                        Err(err) => return Some(Err(err)),
                    };
                    Some(Ok(Array {
                        array_flags: flags,
                        size: size,
                        name: name,
                        data: numeric_data,
                    }))
                }
                _ => None,
            })
            .collect();
        let arrays = arrays?;
        Ok(MatFile {
            arrays: arrays,
            header: Header {
                version: 1,
                mat_identifier: "".to_string(),
                description: "".to_string(),
                byte_order: Endianness::Little,
                subsys_offset: 0,
                deflate_level: 1,
            },
        })
    }
    pub fn save_matfile<T: AsRef<str>>(&self, path: T) -> Result<(), MatError> {
        let mut file = std::fs::File::create(path.as_ref())?;
        let header = writer::write_header(self)?;
        let _r = file.write_all(header.as_ref());
        let body = writer::write_body(self)?;
        let _r = file.write_all(body.as_ref());
        Ok(())
    }
}

mod tests {

    #[test]
    fn write_matfile() -> std::result::Result<(), crate::mat_error::MatError> {
        let mut new_matfile = super::MatFile::new_mat_file();
        let rows = 4;
        let cols = 5;
        let mut matrix = super::MatFile::new_matrix(
            "matrixIdentity",
            rows,
            cols,
            false,
            crate::parse::MatlabType::Int8,
        )?;
        let mut count = 1;
        for i in 0..rows {
            for j in 0..cols {
                matrix.set_int8(i, j, count);
                count += 1;
            }
        }
        println!("matrix==>{:?}", matrix);
        new_matfile.add_array(matrix);
        let _r = new_matfile.save_matfile("d:/newmyfile.mat")?;
        println!("加载生成文件");
        let file = std::fs::File::open("d:/newmyfile.mat")?;
        let matfile = super::MatFile::parse(file)?;
        let array = matfile.find_by_name("matrixIdentity");
        println!("matrixIdentity={:?}", array);
        Ok(())
    }
    #[test]
    fn read_matfile() -> std::result::Result<(), crate::mat_error::MatError> {
        let file = std::fs::File::open("d:/myfile.mat")?;
        let matfile = super::MatFile::parse(file)?;
        let array = matfile.find_by_name("matrixIdentity");
        println!("matrixIdentity={:?}", array);
        let array = matfile.find_by_name("tout");
        println!("tout={:?}", array);
        let array = matfile.find_by_name("x");
        println!("x={:?}", array);
        let array = matfile.find_by_name("y");
        println!("y={:?}", array);
        Ok(())
    }
}
