use std::io;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::protocol::{ThriftReader, ThriftWriter, ttype};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LogicType {
    Boolean = 0,
    TinyInt = 1,
    SmallInt = 2,
    #[default]
    Integer = 3,
    BigInt = 4,
    HugeInt = 5,
    Decimal = 6,
    Float = 7,
    Double = 8,
    Float16 = 9,
    BFloat16 = 10,
    Varchar = 11,
    Embedding = 12,
    Tensor = 13,
    TensorArray = 14,
    Sparse = 15,
    MultiVector = 16,
    Date = 17,
    Time = 18,
    DateTime = 19,
    Timestamp = 20,
    Interval = 21,
    Array = 22,
    Json = 23,
    Invalid = 24,
}

impl LogicType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::Boolean,
            1 => Self::TinyInt,
            2 => Self::SmallInt,
            3 => Self::Integer,
            4 => Self::BigInt,
            5 => Self::HugeInt,
            6 => Self::Decimal,
            7 => Self::Float,
            8 => Self::Double,
            9 => Self::Float16,
            10 => Self::BFloat16,
            11 => Self::Varchar,
            12 => Self::Embedding,
            13 => Self::Tensor,
            14 => Self::TensorArray,
            15 => Self::Sparse,
            16 => Self::MultiVector,
            17 => Self::Date,
            18 => Self::Time,
            19 => Self::DateTime,
            20 => Self::Timestamp,
            21 => Self::Interval,
            22 => Self::Array,
            23 => Self::Json,
            _ => Self::Invalid,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Boolean => "Boolean",
            Self::TinyInt => "TinyInt",
            Self::SmallInt => "SmallInt",
            Self::Integer => "Integer",
            Self::BigInt => "BigInt",
            Self::HugeInt => "HugeInt",
            Self::Decimal => "Decimal",
            Self::Float => "Float",
            Self::Double => "Double",
            Self::Float16 => "Float16",
            Self::BFloat16 => "BFloat16",
            Self::Varchar => "Varchar",
            Self::Embedding => "Embedding",
            Self::Tensor => "Tensor",
            Self::TensorArray => "TensorArray",
            Self::Sparse => "Sparse",
            Self::MultiVector => "MultiVector",
            Self::Date => "Date",
            Self::Time => "Time",
            Self::DateTime => "DateTime",
            Self::Timestamp => "Timestamp",
            Self::Interval => "Interval",
            Self::Array => "Array",
            Self::Json => "Json",
            Self::Invalid => "Invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ElementType {
    ElementBit = 0,
    ElementUInt8 = 1,
    ElementInt8 = 2,
    ElementInt16 = 3,
    ElementInt32 = 4,
    ElementInt64 = 5,
    #[default]
    ElementFloat32 = 6,
    ElementFloat64 = 7,
    ElementFloat16 = 8,
    ElementBFloat16 = 9,
}

impl ElementType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::ElementBit,
            1 => Self::ElementUInt8,
            2 => Self::ElementInt8,
            3 => Self::ElementInt16,
            4 => Self::ElementInt32,
            5 => Self::ElementInt64,
            6 => Self::ElementFloat32,
            7 => Self::ElementFloat64,
            8 => Self::ElementFloat16,
            9 => Self::ElementBFloat16,
            _ => Self::ElementFloat32,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ElementBit => "bit",
            Self::ElementUInt8 => "uint8",
            Self::ElementInt8 => "int8",
            Self::ElementInt16 => "int16",
            Self::ElementInt32 => "int32",
            Self::ElementInt64 => "int64",
            Self::ElementFloat32 => "float32",
            Self::ElementFloat64 => "float64",
            Self::ElementFloat16 => "float16",
            Self::ElementBFloat16 => "bfloat16",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ConstraintType {
    #[default]
    PrimaryKey = 0,
    NotNull = 1,
    Null = 2,
    Unique = 3,
}

impl ConstraintType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::PrimaryKey,
            1 => Self::NotNull,
            2 => Self::Null,
            3 => Self::Unique,
            _ => Self::Null,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CreateConflict {
    Ignore = 0,
    #[default]
    Error = 1,
    Replace = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DropConflict {
    Ignore = 0,
    #[default]
    Error = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ColumnType {
    ColumnBool = 0,
    ColumnInt8 = 1,
    ColumnInt16 = 2,
    ColumnInt32 = 3,
    ColumnInt64 = 4,
    ColumnFloat32 = 5,
    ColumnFloat64 = 6,
    ColumnFloat16 = 7,
    ColumnBFloat16 = 8,
    ColumnVarchar = 9,
    ColumnEmbedding = 10,
    ColumnTensor = 11,
    ColumnTensorArray = 12,
    ColumnSparse = 13,
    ColumnMultiVector = 14,
    ColumnRowID = 15,
    ColumnDate = 16,
    ColumnTime = 17,
    ColumnDateTime = 18,
    ColumnTimestamp = 19,
    ColumnInterval = 20,
    ColumnArray = 21,
    ColumnJson = 22,
    #[default]
    ColumnInvalid = 23,
}

impl ColumnType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::ColumnBool,
            1 => Self::ColumnInt8,
            2 => Self::ColumnInt16,
            3 => Self::ColumnInt32,
            4 => Self::ColumnInt64,
            5 => Self::ColumnFloat32,
            6 => Self::ColumnFloat64,
            7 => Self::ColumnFloat16,
            8 => Self::ColumnBFloat16,
            9 => Self::ColumnVarchar,
            10 => Self::ColumnEmbedding,
            11 => Self::ColumnTensor,
            12 => Self::ColumnTensorArray,
            13 => Self::ColumnSparse,
            14 => Self::ColumnMultiVector,
            15 => Self::ColumnRowID,
            16 => Self::ColumnDate,
            17 => Self::ColumnTime,
            18 => Self::ColumnDateTime,
            19 => Self::ColumnTimestamp,
            20 => Self::ColumnInterval,
            21 => Self::ColumnArray,
            22 => Self::ColumnJson,
            _ => Self::ColumnInvalid,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ColumnBool => "Boolean",
            Self::ColumnInt8 => "TinyInt",
            Self::ColumnInt16 => "SmallInt",
            Self::ColumnInt32 => "Integer",
            Self::ColumnInt64 => "BigInt",
            Self::ColumnFloat32 => "Float32",
            Self::ColumnFloat64 => "Float64",
            Self::ColumnFloat16 => "Float16",
            Self::ColumnBFloat16 => "BFloat16",
            Self::ColumnVarchar => "Varchar",
            Self::ColumnEmbedding => "Embedding",
            Self::ColumnTensor => "Tensor",
            Self::ColumnTensorArray => "TensorArray",
            Self::ColumnSparse => "Sparse",
            Self::ColumnMultiVector => "MultiVector",
            Self::ColumnRowID => "RowID",
            Self::ColumnDate => "Date",
            Self::ColumnTime => "Time",
            Self::ColumnDateTime => "DateTime",
            Self::ColumnTimestamp => "Timestamp",
            Self::ColumnInterval => "Interval",
            Self::ColumnArray => "Array",
            Self::ColumnJson => "Json",
            Self::ColumnInvalid => "Invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LiteralType {
    Boolean = 0,
    Double = 1,
    String = 2,
    #[default]
    Int64 = 3,
    Null = 4,
    Date = 5,
    Time = 6,
    DateTime = 7,
    Timestamp = 8,
    IntegerArray = 9,
    DoubleArray = 10,
    SubQuery = 11,
    Interval = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CommandType {
    #[default]
    Use = 0,
    Set = 1,
    Export = 2,
    CheckTable = 3,
    Compact = 4,
    Lock = 5,
    Unlock = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ExplainType {
    #[default]
    Analyze = 0,
    Ast = 1,
    UnOpt = 2,
    Opt = 3,
    Physical = 4,
    Pipeline = 5,
    Fragment = 6,
}

// ----- Complex Structs -----

#[derive(Debug, Clone, Default)]
pub struct DataType {
    pub logic_type: LogicType,
    pub physical_type: Option<i32>,
    pub max_scale: Option<i32>,
    pub max_precision: Option<i32>,
    pub length: Option<i32>,
    pub dimension: Option<i32>,
    pub element_type: Option<ElementType>,
    pub element_data_type: Option<Box<DataType>>,
    pub element_dimension: Option<i32>,
    pub element_element_type: Option<ElementType>,
}

impl DataType {
    pub fn simple(logic_type: LogicType) -> Self {
        Self {
            logic_type,
            ..Default::default()
        }
    }

    pub fn embedding(element_type: ElementType, dimension: i32) -> Self {
        Self {
            logic_type: LogicType::Embedding,
            element_type: Some(element_type),
            dimension: Some(dimension),
            ..Default::default()
        }
    }

    pub fn varchar(length: i32) -> Self {
        Self {
            logic_type: LogicType::Varchar,
            length: Some(length),
            ..Default::default()
        }
    }

    pub fn format_type(&self) -> String {
        match self.logic_type {
            LogicType::Embedding => {
                let elem = self
                    .element_type
                    .map(|e| e.display_name())
                    .unwrap_or("float32");
                let dim = self.dimension.unwrap_or(0);
                format!("Embedding({elem}, {dim})")
            }
            LogicType::Varchar => match self.length {
                Some(len) if len > 0 => format!("Varchar({len})"),
                _ => "Varchar".to_string(),
            },
            LogicType::Array => {
                if let Some(inner) = &self.element_data_type {
                    format!("Array({})", inner.format_type())
                } else {
                    "Array".to_string()
                }
            }
            _ => self.logic_type.display_name().to_string(),
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("DataType").await?;

        // 1: logic_type
        writer
            .write_field_begin("logic_type", ttype::I32, 1)
            .await?;
        writer.write_i32(self.logic_type as i32).await?;
        writer.write_field_end().await?;

        // 2: physical_type (union)
        writer
            .write_field_begin("physical_type", ttype::STRUCT, 2)
            .await?;
        writer.write_struct_begin("PhysicalType").await?;
        match self.logic_type {
            LogicType::Embedding => {
                writer
                    .write_field_begin("embedding_type", ttype::STRUCT, 3)
                    .await?;
                writer.write_struct_begin("EmbeddingType").await?;
                writer.write_field_begin("dimension", ttype::I32, 1).await?;
                writer.write_i32(self.dimension.unwrap_or(0)).await?;
                writer.write_field_end().await?;
                writer
                    .write_field_begin("element_type", ttype::I32, 2)
                    .await?;
                writer
                    .write_i32(self.element_type.unwrap_or_default() as i32)
                    .await?;
                writer.write_field_end().await?;
                writer.write_field_stop().await?;
                writer.write_struct_end().await?;
                writer.write_field_end().await?;
            }
            LogicType::Varchar => {
                writer
                    .write_field_begin("varchar_type", ttype::STRUCT, 2)
                    .await?;
                writer.write_struct_begin("VarcharType").await?;
                writer.write_field_stop().await?;
                writer.write_struct_end().await?;
                writer.write_field_end().await?;
            }
            _ => {
                writer
                    .write_field_begin("number_type", ttype::STRUCT, 1)
                    .await?;
                writer.write_struct_begin("NumberType").await?;
                writer.write_field_stop().await?;
                writer.write_struct_end().await?;
                writer.write_field_end().await?;
            }
        }
        writer.write_field_stop().await?;
        writer.write_struct_end().await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut data_type = DataType::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => data_type.logic_type = LogicType::from_i32(reader.read_i32().await?),
                2 => {
                    // PhysicalType union
                    reader.read_struct_begin().await?;
                    loop {
                        let (p_type, p_id) = reader.read_field_begin().await?;
                        if p_type == ttype::STOP {
                            break;
                        }
                        match p_id {
                            3 => {
                                // EmbeddingType
                                reader.read_struct_begin().await?;
                                loop {
                                    let (e_type, e_id) = reader.read_field_begin().await?;
                                    if e_type == ttype::STOP {
                                        break;
                                    }
                                    match e_id {
                                        1 => data_type.dimension = Some(reader.read_i32().await?),
                                        2 => {
                                            data_type.element_type = Some(ElementType::from_i32(
                                                reader.read_i32().await?,
                                            ))
                                        }
                                        _ => reader.skip(e_type, 0).await?,
                                    }
                                    reader.read_field_end().await?;
                                }
                                reader.read_struct_end().await?;
                            }
                            _ => reader.skip(p_type, 0).await?,
                        }
                        reader.read_field_end().await?;
                    }
                    reader.read_struct_end().await?;
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(data_type)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConstantExpr {
    pub literal_type: LiteralType,
    pub bool_value: Option<bool>,
    pub i64_value: Option<i64>,
    pub f64_value: Option<f64>,
    pub str_value: Option<String>,
    pub i64_array_value: Option<Vec<i64>>,
    pub f64_array_value: Option<Vec<f64>>,
}

impl ConstantExpr {
    pub fn int(v: i64) -> Self {
        Self {
            literal_type: LiteralType::Int64,
            i64_value: Some(v),
            ..Default::default()
        }
    }

    pub fn float(v: f64) -> Self {
        Self {
            literal_type: LiteralType::Double,
            f64_value: Some(v),
            ..Default::default()
        }
    }

    pub fn string(s: impl Into<String>) -> Self {
        Self {
            literal_type: LiteralType::String,
            str_value: Some(s.into()),
            ..Default::default()
        }
    }

    pub fn bool(b: bool) -> Self {
        Self {
            literal_type: LiteralType::Boolean,
            bool_value: Some(b),
            ..Default::default()
        }
    }

    pub fn null() -> Self {
        Self {
            literal_type: LiteralType::Null,
            ..Default::default()
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("ConstantExpr").await?;

        // 1: literal_type
        writer
            .write_field_begin("literal_type", ttype::I32, 1)
            .await?;
        writer.write_i32(self.literal_type as i32).await?;
        writer.write_field_end().await?;

        if let Some(b) = self.bool_value {
            writer
                .write_field_begin("bool_value", ttype::BOOL, 2)
                .await?;
            writer.write_bool(b).await?;
            writer.write_field_end().await?;
        }

        if let Some(i) = self.i64_value {
            writer.write_field_begin("i64_value", ttype::I64, 3).await?;
            writer.write_i64(i).await?;
            writer.write_field_end().await?;
        }

        if let Some(f) = self.f64_value {
            writer
                .write_field_begin("f64_value", ttype::DOUBLE, 4)
                .await?;
            writer.write_double(f).await?;
            writer.write_field_end().await?;
        }

        if let Some(s) = &self.str_value {
            writer
                .write_field_begin("str_value", ttype::STRING, 5)
                .await?;
            writer.write_string(s).await?;
            writer.write_field_end().await?;
        }

        if let Some(arr) = &self.i64_array_value {
            writer
                .write_field_begin("i64_array_value", ttype::LIST, 6)
                .await?;
            writer.write_list_begin(ttype::I64, arr.len()).await?;
            for item in arr {
                writer.write_i64(*item).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(arr) = &self.f64_array_value {
            writer
                .write_field_begin("f64_array_value", ttype::LIST, 7)
                .await?;
            writer.write_list_begin(ttype::DOUBLE, arr.len()).await?;
            for item in arr {
                writer.write_double(*item).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut expr = ConstantExpr::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => {
                    let v = reader.read_i32().await?;
                    expr.literal_type = match v {
                        0 => LiteralType::Boolean,
                        1 => LiteralType::Double,
                        2 => LiteralType::String,
                        3 => LiteralType::Int64,
                        4 => LiteralType::Null,
                        5 => LiteralType::Date,
                        6 => LiteralType::Time,
                        7 => LiteralType::DateTime,
                        8 => LiteralType::Timestamp,
                        9 => LiteralType::IntegerArray,
                        10 => LiteralType::DoubleArray,
                        11 => LiteralType::SubQuery,
                        12 => LiteralType::Interval,
                        _ => LiteralType::Int64,
                    };
                }
                2 => expr.bool_value = Some(reader.read_bool().await?),
                3 => expr.i64_value = Some(reader.read_i64().await?),
                4 => expr.f64_value = Some(reader.read_double().await?),
                5 => expr.str_value = Some(reader.read_string().await?),
                6 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut arr = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        arr.push(reader.read_i64().await?);
                    }
                    reader.read_list_end().await?;
                    expr.i64_array_value = Some(arr);
                }
                7 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut arr = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        arr.push(reader.read_double().await?);
                    }
                    reader.read_list_end().await?;
                    expr.f64_array_value = Some(arr);
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(expr)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ColumnExpr {
    pub column_name: Vec<String>,
    pub star: bool,
}

impl ColumnExpr {
    pub fn single(name: impl Into<String>) -> Self {
        Self {
            column_name: vec![name.into()],
            star: false,
        }
    }

    pub fn star() -> Self {
        Self {
            column_name: Vec::new(),
            star: true,
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("ColumnExpr").await?;

        // 1: column_name list<string>
        writer
            .write_field_begin("column_name", ttype::LIST, 1)
            .await?;
        writer
            .write_list_begin(ttype::STRING, self.column_name.len())
            .await?;
        for col in &self.column_name {
            writer.write_string(col).await?;
        }
        writer.write_list_end().await?;
        writer.write_field_end().await?;

        // 2: star bool
        writer.write_field_begin("star", ttype::BOOL, 2).await?;
        writer.write_bool(self.star).await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut col_expr = ColumnExpr::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut cols = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        cols.push(reader.read_string().await?);
                    }
                    reader.read_list_end().await?;
                    col_expr.column_name = cols;
                }
                2 => col_expr.star = reader.read_bool().await?,
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(col_expr)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionExpr {
    pub function_name: String,
    pub arguments: Option<Vec<ParsedExpr>>,
}

impl FunctionExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("FunctionExpr").await?;

        // 1: function_name
        writer
            .write_field_begin("function_name", ttype::STRING, 1)
            .await?;
        writer.write_string(&self.function_name).await?;
        writer.write_field_end().await?;

        // 2: arguments
        if let Some(args) = &self.arguments {
            writer
                .write_field_begin("arguments", ttype::LIST, 2)
                .await?;
            writer.write_list_begin(ttype::STRUCT, args.len()).await?;
            for arg in args {
                Box::pin(arg.write(writer)).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut func = FunctionExpr::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => func.function_name = reader.read_string().await?,
                2 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut args = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        args.push(Box::pin(ParsedExpr::read(reader)).await?);
                    }
                    reader.read_list_end().await?;
                    func.arguments = Some(args);
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(func)
    }
}

#[derive(Debug, Clone, Default)]
pub struct BetweenExpr {
    pub value: Box<ParsedExpr>,
    pub upper_bound: Box<ParsedExpr>,
    pub lower_bound: Box<ParsedExpr>,
}

impl BetweenExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("BetweenExpr").await?;
        writer.write_field_begin("value", ttype::STRUCT, 1).await?;
        Box::pin(self.value.write(writer)).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("upper_bound", ttype::STRUCT, 2)
            .await?;
        Box::pin(self.upper_bound.write(writer)).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("lower_bound", ttype::STRUCT, 3)
            .await?;
        Box::pin(self.lower_bound.write(writer)).await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct InitParameter {
    pub param_name: String,
    pub param_value: String,
}

impl InitParameter {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("InitParameter").await?;
        writer
            .write_field_begin("param_name", ttype::STRING, 1)
            .await?;
        writer.write_string(&self.param_name).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("param_value", ttype::STRING, 2)
            .await?;
        writer.write_string(&self.param_value).await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct MatchExpr {
    pub fields: String,
    pub matching_text: String,
    pub options_text: String,
    pub opt_params: Option<Vec<InitParameter>>,
    pub topn: Option<i64>,
}

impl MatchExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("MatchExpr").await?;
        writer.write_field_begin("fields", ttype::STRING, 1).await?;
        writer.write_string(&self.fields).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("matching_text", ttype::STRING, 2)
            .await?;
        writer.write_string(&self.matching_text).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("options_text", ttype::STRING, 3)
            .await?;
        writer.write_string(&self.options_text).await?;
        writer.write_field_end().await?;

        if let Some(params) = &self.opt_params {
            writer
                .write_field_begin("opt_params", ttype::LIST, 4)
                .await?;
            writer.write_list_begin(ttype::STRUCT, params.len()).await?;
            for p in params {
                p.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(topn) = self.topn {
            writer.write_field_begin("topn", ttype::I64, 5).await?;
            writer.write_i64(topn).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct MatchDenseExpr {
    pub column_expr: Box<ParsedExpr>,
    pub embedding_data: Vec<u8>,
    pub embedding_data_type: ElementType,
    pub distance_type: String,
    pub topn: i64,
    pub opt_params: Option<Vec<InitParameter>>,
}

impl MatchDenseExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("MatchDenseExpr").await?;
        writer
            .write_field_begin("column_expr", ttype::STRUCT, 1)
            .await?;
        Box::pin(self.column_expr.write(writer)).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("embedding_data", ttype::STRING, 2)
            .await?;
        writer.write_binary(&self.embedding_data).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("embedding_data_type", ttype::I32, 3)
            .await?;
        writer.write_i32(self.embedding_data_type as i32).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("distance_type", ttype::STRING, 4)
            .await?;
        writer.write_string(&self.distance_type).await?;
        writer.write_field_end().await?;

        writer.write_field_begin("topn", ttype::I64, 5).await?;
        writer.write_i64(self.topn).await?;
        writer.write_field_end().await?;

        if let Some(params) = &self.opt_params {
            writer
                .write_field_begin("opt_params", ttype::LIST, 6)
                .await?;
            writer.write_list_begin(ttype::STRUCT, params.len()).await?;
            for p in params {
                p.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct MatchSparseExpr {
    pub column_expr: Box<ParsedExpr>,
    pub query_sparse_expr: ConstantExpr,
    pub metric_type: String,
    pub topn: i64,
    pub opt_params: Option<Vec<InitParameter>>,
}

impl MatchSparseExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("MatchSparseExpr").await?;
        writer
            .write_field_begin("column_expr", ttype::STRUCT, 1)
            .await?;
        Box::pin(self.column_expr.write(writer)).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("query_sparse_expr", ttype::STRUCT, 2)
            .await?;
        self.query_sparse_expr.write(writer).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("metric_type", ttype::STRING, 3)
            .await?;
        writer.write_string(&self.metric_type).await?;
        writer.write_field_end().await?;

        writer.write_field_begin("topn", ttype::I64, 4).await?;
        writer.write_i64(self.topn).await?;
        writer.write_field_end().await?;

        if let Some(params) = &self.opt_params {
            writer
                .write_field_begin("opt_params", ttype::LIST, 5)
                .await?;
            writer.write_list_begin(ttype::STRUCT, params.len()).await?;
            for p in params {
                p.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct MatchTensorExpr {
    pub column_expr: Box<ParsedExpr>,
    pub embedding_data: Vec<u8>,
    pub embedding_data_type: ElementType,
    pub method_type: String,
    pub topn: i64,
    pub opt_params: Option<Vec<InitParameter>>,
}

impl MatchTensorExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("MatchTensorExpr").await?;
        writer
            .write_field_begin("column_expr", ttype::STRUCT, 1)
            .await?;
        Box::pin(self.column_expr.write(writer)).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("embedding_data", ttype::STRING, 2)
            .await?;
        writer.write_binary(&self.embedding_data).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("embedding_data_type", ttype::I32, 3)
            .await?;
        writer.write_i32(self.embedding_data_type as i32).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("method_type", ttype::STRING, 4)
            .await?;
        writer.write_string(&self.method_type).await?;
        writer.write_field_end().await?;

        writer.write_field_begin("topn", ttype::I64, 5).await?;
        writer.write_i64(self.topn).await?;
        writer.write_field_end().await?;

        if let Some(params) = &self.opt_params {
            writer
                .write_field_begin("opt_params", ttype::LIST, 6)
                .await?;
            writer.write_list_begin(ttype::STRUCT, params.len()).await?;
            for p in params {
                p.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct FusionExpr {
    pub method: String,
    pub options_text: String,
    pub opt_params: Option<Vec<InitParameter>>,
    pub topn: Option<i64>,
}

impl FusionExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("FusionExpr").await?;
        writer.write_field_begin("method", ttype::STRING, 1).await?;
        writer.write_string(&self.method).await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("options_text", ttype::STRING, 2)
            .await?;
        writer.write_string(&self.options_text).await?;
        writer.write_field_end().await?;

        if let Some(params) = &self.opt_params {
            writer
                .write_field_begin("opt_params", ttype::LIST, 3)
                .await?;
            writer.write_list_begin(ttype::STRUCT, params.len()).await?;
            for p in params {
                p.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(topn) = self.topn {
            writer.write_field_begin("topn", ttype::I64, 4).await?;
            writer.write_i64(topn).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchExpr {
    pub match_exprs: Option<Vec<MatchExpr>>,
    pub match_dense_exprs: Option<Vec<MatchDenseExpr>>,
    pub match_sparse_exprs: Option<Vec<MatchSparseExpr>>,
    pub match_tensor_exprs: Option<Vec<MatchTensorExpr>>,
    pub fusion_expr: Option<FusionExpr>,
}

impl SearchExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("SearchExpr").await?;

        if let Some(exprs) = &self.match_exprs {
            writer
                .write_field_begin("match_exprs", ttype::LIST, 1)
                .await?;
            writer.write_list_begin(ttype::STRUCT, exprs.len()).await?;
            for e in exprs {
                e.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(exprs) = &self.match_dense_exprs {
            writer
                .write_field_begin("match_dense_exprs", ttype::LIST, 2)
                .await?;
            writer.write_list_begin(ttype::STRUCT, exprs.len()).await?;
            for e in exprs {
                e.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(exprs) = &self.match_sparse_exprs {
            writer
                .write_field_begin("match_sparse_exprs", ttype::LIST, 3)
                .await?;
            writer.write_list_begin(ttype::STRUCT, exprs.len()).await?;
            for e in exprs {
                e.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(exprs) = &self.match_tensor_exprs {
            writer
                .write_field_begin("match_tensor_exprs", ttype::LIST, 4)
                .await?;
            writer.write_list_begin(ttype::STRUCT, exprs.len()).await?;
            for e in exprs {
                e.write(writer).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        if let Some(fusion) = &self.fusion_expr {
            writer
                .write_field_begin("fusion_expr", ttype::STRUCT, 5)
                .await?;
            fusion.write(writer).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParsedExpr {
    pub constant_expr: Option<ConstantExpr>,
    pub column_expr: Option<ColumnExpr>,
    pub function_expr: Option<FunctionExpr>,
    pub between_expr: Option<Box<BetweenExpr>>,
    pub match_expr: Option<Box<MatchExpr>>,
    pub match_dense_expr: Option<Box<MatchDenseExpr>>,
    pub match_sparse_expr: Option<Box<MatchSparseExpr>>,
    pub search_expr: Option<Box<SearchExpr>>,
    pub fusion_expr: Option<Box<FusionExpr>>,
    pub match_tensor_expr: Option<Box<MatchTensorExpr>>,
    pub alias_name: Option<String>,
}

impl ParsedExpr {
    pub fn constant(c: ConstantExpr) -> Self {
        Self {
            constant_expr: Some(c),
            ..Default::default()
        }
    }

    pub fn column(col: ColumnExpr) -> Self {
        Self {
            column_expr: Some(col),
            ..Default::default()
        }
    }

    pub fn function(f: FunctionExpr) -> Self {
        Self {
            function_expr: Some(f),
            ..Default::default()
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("ParsedExpr").await?;

        // 1: type (ParsedExprType union)
        writer.write_field_begin("type", ttype::STRUCT, 1).await?;
        writer.write_struct_begin("ParsedExprType").await?;

        if let Some(c) = &self.constant_expr {
            writer
                .write_field_begin("constant_expr", ttype::STRUCT, 1)
                .await?;
            c.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(c) = &self.column_expr {
            writer
                .write_field_begin("column_expr", ttype::STRUCT, 2)
                .await?;
            c.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(f) = &self.function_expr {
            writer
                .write_field_begin("function_expr", ttype::STRUCT, 3)
                .await?;
            Box::pin(f.write(writer)).await?;
            writer.write_field_end().await?;
        } else if let Some(b) = &self.between_expr {
            writer
                .write_field_begin("between_expr", ttype::STRUCT, 4)
                .await?;
            b.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(m) = &self.match_sparse_expr {
            writer
                .write_field_begin("match_sparse_expr", ttype::STRUCT, 6)
                .await?;
            m.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(m) = &self.match_tensor_expr {
            writer
                .write_field_begin("match_tensor_expr", ttype::STRUCT, 7)
                .await?;
            m.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(m) = &self.match_expr {
            writer
                .write_field_begin("match_expr", ttype::STRUCT, 8)
                .await?;
            m.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(f) = &self.fusion_expr {
            writer
                .write_field_begin("fusion_expr", ttype::STRUCT, 9)
                .await?;
            f.write(writer).await?;
            writer.write_field_end().await?;
        } else if let Some(s) = &self.search_expr {
            writer
                .write_field_begin("search_expr", ttype::STRUCT, 10)
                .await?;
            s.write(writer).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await?;
        writer.write_field_end().await?;

        // 2: alias_name
        if let Some(alias) = &self.alias_name {
            writer
                .write_field_begin("alias_name", ttype::STRING, 2)
                .await?;
            writer.write_string(alias).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut expr = ParsedExpr::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => {
                    // ParsedExprType union
                    reader.read_struct_begin().await?;
                    loop {
                        let (u_type, u_id) = reader.read_field_begin().await?;
                        if u_type == ttype::STOP {
                            break;
                        }
                        match u_id {
                            1 => expr.constant_expr = Some(ConstantExpr::read(reader).await?),
                            2 => expr.column_expr = Some(ColumnExpr::read(reader).await?),
                            3 => {
                                expr.function_expr =
                                    Some(Box::pin(FunctionExpr::read(reader)).await?)
                            }
                            _ => reader.skip(u_type, 0).await?,
                        }
                        reader.read_field_end().await?;
                    }
                    reader.read_struct_end().await?;
                }
                2 => expr.alias_name = Some(reader.read_string().await?),
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(expr)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrderByExpr {
    pub expr: ParsedExpr,
    pub asc: bool,
}

impl OrderByExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("OrderByExpr").await?;
        writer.write_field_begin("expr", ttype::STRUCT, 1).await?;
        Box::pin(self.expr.write(writer)).await?;
        writer.write_field_end().await?;

        writer.write_field_begin("asc", ttype::BOOL, 2).await?;
        writer.write_bool(self.asc).await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct ColumnDef {
    pub id: i64,
    pub data_type: DataType,
    pub name: String,
    pub constraints: Vec<ConstraintType>,
    pub constant_expr: Option<ConstantExpr>,
    pub comment: Option<String>,
}

impl ColumnDef {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("ColumnDef").await?;

        // 1: id
        writer.write_field_begin("id", ttype::I32, 1).await?;
        writer.write_i32(self.id as i32).await?;
        writer.write_field_end().await?;

        // 2: name
        writer.write_field_begin("name", ttype::STRING, 2).await?;
        writer.write_string(&self.name).await?;
        writer.write_field_end().await?;

        // 3: data_type
        writer
            .write_field_begin("data_type", ttype::STRUCT, 3)
            .await?;
        Box::pin(self.data_type.write(writer)).await?;
        writer.write_field_end().await?;

        // 4: constraints
        if !self.constraints.is_empty() {
            writer
                .write_field_begin("constraints", ttype::LIST, 4)
                .await?;
            writer
                .write_list_begin(ttype::I32, self.constraints.len())
                .await?;
            for c in &self.constraints {
                writer.write_i32(*c as i32).await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        // 5: constant_expr
        if let Some(c) = &self.constant_expr {
            writer
                .write_field_begin("constant_expr", ttype::STRUCT, 5)
                .await?;
            c.write(writer).await?;
            writer.write_field_end().await?;
        }

        // 6: comment
        if let Some(comm) = &self.comment {
            writer
                .write_field_begin("comment", ttype::STRING, 6)
                .await?;
            writer.write_string(comm).await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut def = ColumnDef::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => def.id = reader.read_i32().await? as i64,
                2 => def.name = reader.read_string().await?,
                3 => def.data_type = DataType::read(reader).await?,
                4 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut cons = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        let c = reader.read_i32().await?;
                        cons.push(ConstraintType::from_i32(c));
                    }
                    reader.read_list_end().await?;
                    def.constraints = cons;
                }
                5 => def.constant_expr = Some(ConstantExpr::read(reader).await?),
                6 => def.comment = Some(reader.read_string().await?),
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(def)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CreateOption {
    pub conflict_type: CreateConflict,
    pub properties: Vec<(String, String)>,
}

impl CreateOption {
    pub fn new(conflict_type: CreateConflict) -> Self {
        Self {
            conflict_type,
            properties: Vec::new(),
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("CreateOption").await?;
        writer
            .write_field_begin("conflict_type", ttype::I32, 1)
            .await?;
        writer.write_i32(self.conflict_type as i32).await?;
        writer.write_field_end().await?;

        if !self.properties.is_empty() {
            writer
                .write_field_begin("properties", ttype::LIST, 2)
                .await?;
            writer
                .write_list_begin(ttype::STRUCT, self.properties.len())
                .await?;
            for (k, v) in &self.properties {
                writer.write_struct_begin("Property").await?;
                writer.write_field_begin("key", ttype::STRING, 1).await?;
                writer.write_string(k).await?;
                writer.write_field_end().await?;
                writer.write_field_begin("value", ttype::STRING, 2).await?;
                writer.write_string(v).await?;
                writer.write_field_end().await?;
                writer.write_field_stop().await?;
                writer.write_struct_end().await?;
            }
            writer.write_list_end().await?;
            writer.write_field_end().await?;
        }

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct DropOption {
    pub conflict_type: DropConflict,
}

impl DropOption {
    pub fn new(conflict_type: DropConflict) -> Self {
        Self { conflict_type }
    }

    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("DropOption").await?;
        writer
            .write_field_begin("conflict_type", ttype::I32, 1)
            .await?;
        writer.write_i32(self.conflict_type as i32).await?;
        writer.write_field_end().await?;
        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct Field {
    pub column_names: Vec<String>,
    pub parse_exprs: Vec<ParsedExpr>,
}

impl Field {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("Field").await?;
        writer
            .write_field_begin("column_names", ttype::LIST, 1)
            .await?;
        writer
            .write_list_begin(ttype::STRING, self.column_names.len())
            .await?;
        for col in &self.column_names {
            writer.write_string(col).await?;
        }
        writer.write_list_end().await?;
        writer.write_field_end().await?;

        writer
            .write_field_begin("parse_exprs", ttype::LIST, 2)
            .await?;
        writer
            .write_list_begin(ttype::STRUCT, self.parse_exprs.len())
            .await?;
        for expr in &self.parse_exprs {
            expr.write(writer).await?;
        }
        writer.write_list_end().await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpdateExpr {
    pub column_name: String,
    pub value: ParsedExpr,
}

impl UpdateExpr {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("UpdateExpr").await?;
        writer
            .write_field_begin("column_name", ttype::STRING, 1)
            .await?;
        writer.write_string(&self.column_name).await?;
        writer.write_field_end().await?;

        writer.write_field_begin("value", ttype::STRUCT, 2).await?;
        Box::pin(self.value.write(writer)).await?;
        writer.write_field_end().await?;

        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct UseCmd {
    pub db_name: String,
}

impl UseCmd {
    pub async fn write<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut ThriftWriter<W>,
    ) -> io::Result<()> {
        writer.write_struct_begin("UseCmd").await?;
        writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        writer.write_string(&self.db_name).await?;
        writer.write_field_end().await?;
        writer.write_field_stop().await?;
        writer.write_struct_end().await
    }
}

// ----- Responses & Field Types -----

#[derive(Debug, Clone, Default)]
pub struct CommonResponse {
    pub error_code: i64,
    pub error_msg: Option<String>,
    pub session_id: i64,
}

impl CommonResponse {
    pub fn is_ok(&self) -> bool {
        self.error_code == 0
    }

    pub fn error_message(&self) -> String {
        match &self.error_msg {
            Some(msg) if !msg.is_empty() => format!("{msg} (code {})", self.error_code),
            _ => format!("Infinity error code {}", self.error_code),
        }
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut resp = CommonResponse::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => resp.error_code = reader.read_i64().await?,
                2 => resp.error_msg = Some(reader.read_string().await?),
                3 => resp.session_id = reader.read_i64().await?,
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(resp)
    }
}

pub type ConnectResponse = CommonResponse;

#[derive(Debug, Clone, Default)]
pub struct ListDatabaseResponse {
    pub error_code: i64,
    pub error_msg: Option<String>,
    pub db_names: Option<Vec<String>>,
}

impl ListDatabaseResponse {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut resp = ListDatabaseResponse::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => resp.error_code = reader.read_i64().await?,
                2 => resp.error_msg = Some(reader.read_string().await?),
                3 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut list = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        list.push(reader.read_string().await?);
                    }
                    reader.read_list_end().await?;
                    resp.db_names = Some(list);
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListTableResponse {
    pub error_code: i64,
    pub error_msg: Option<String>,
    pub table_names: Option<Vec<String>>,
}

impl ListTableResponse {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut resp = ListTableResponse::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => resp.error_code = reader.read_i64().await?,
                2 => resp.error_msg = Some(reader.read_string().await?),
                3 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut list = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        list.push(reader.read_string().await?);
                    }
                    reader.read_list_end().await?;
                    resp.table_names = Some(list);
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ColumnField {
    pub column_type: ColumnType,
    pub column_vectors: Vec<Vec<u8>>,
    pub column_name: String,
    pub bitmasks: Vec<bool>,
}

impl ColumnField {
    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut field = ColumnField::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => field.column_type = ColumnType::from_i32(reader.read_i32().await?),
                2 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut vecs = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        vecs.push(reader.read_binary().await?);
                    }
                    reader.read_list_end().await?;
                    field.column_vectors = vecs;
                }
                3 => field.column_name = reader.read_string().await?,
                4 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut masks = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        masks.push(reader.read_bool().await?);
                    }
                    reader.read_list_end().await?;
                    field.bitmasks = masks;
                }
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(field)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectResponse {
    pub error_code: i64,
    pub error_msg: Option<String>,
    pub column_defs: Option<Vec<ColumnDef>>,
    pub column_fields: Option<Vec<ColumnField>>,
    pub extra_result: Option<String>,
}

impl SelectResponse {
    pub fn is_ok(&self) -> bool {
        self.error_code == 0
    }

    pub fn error_message(&self) -> String {
        match &self.error_msg {
            Some(msg) if !msg.is_empty() => format!("{msg} (code {})", self.error_code),
            _ => format!("Infinity error code {}", self.error_code),
        }
    }

    pub async fn read<R: AsyncRead + Unpin>(reader: &mut ThriftReader<R>) -> io::Result<Self> {
        let mut resp = SelectResponse::default();
        reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            match f_id {
                1 => resp.error_code = reader.read_i64().await?,
                2 => resp.error_msg = Some(reader.read_string().await?),
                3 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut defs = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        defs.push(ColumnDef::read(reader).await?);
                    }
                    reader.read_list_end().await?;
                    resp.column_defs = Some(defs);
                }
                4 => {
                    let (_, sz) = reader.read_list_begin().await?;
                    let mut cols = Vec::with_capacity(sz);
                    for _ in 0..sz {
                        cols.push(ColumnField::read(reader).await?);
                    }
                    reader.read_list_end().await?;
                    resp.column_fields = Some(cols);
                }
                5 => resp.extra_result = Some(reader.read_string().await?),
                _ => reader.skip(f_type, 0).await?,
            }
            reader.read_field_end().await?;
        }
        reader.read_struct_end().await?;
        Ok(resp)
    }
}
