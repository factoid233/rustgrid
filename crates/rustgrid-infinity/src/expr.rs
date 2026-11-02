use rustgrid_core::{FilterCondition, FilterConjunction, FilterNode, FilterOperator};
use sqlparser::ast::{
    BinaryOperator, ColumnOption, DataType as SqlDataType, Expr as SqlExpr, FunctionArg,
    FunctionArgExpr, FunctionArguments, LimitClause, OrderByKind, Query, SelectItem, SetExpr,
    Statement, TableFactor, Value,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::types::{
    BetweenExpr, ColumnDef, ColumnExpr, ConstantExpr, ConstraintType, DataType, ElementType,
    FunctionExpr, LogicType, MatchDenseExpr, MatchExpr, MatchSparseExpr, MatchTensorExpr,
    OrderByExpr, ParsedExpr, SearchExpr, UpdateExpr,
};

pub fn parse_filter(filter: &[FilterNode]) -> Option<ParsedExpr> {
    let mut current: Option<ParsedExpr> = None;

    for node in filter {
        if !node.enabled() {
            continue;
        }
        let expr = match node {
            FilterNode::Condition(cond) => condition_to_expr(cond),
            FilterNode::Group(grp) => parse_filter(&grp.children),
        };

        let Some(expr) = expr else {
            continue;
        };

        current = match current {
            None => Some(expr),
            Some(prev) => {
                let func = match node.conjunction() {
                    FilterConjunction::And => "and",
                    FilterConjunction::Or => "or",
                };
                Some(ParsedExpr::function(FunctionExpr {
                    function_name: func.to_string(),
                    arguments: Some(vec![prev, expr]),
                }))
            }
        };
    }

    current
}

fn condition_to_expr(cond: &FilterCondition) -> Option<ParsedExpr> {
    let col = ParsedExpr::column(ColumnExpr::single(&cond.column));

    match cond.operator {
        FilterOperator::IsNull => Some(ParsedExpr::function(FunctionExpr {
            function_name: "is_null".to_string(),
            arguments: Some(vec![col]),
        })),
        FilterOperator::IsNotNull => Some(ParsedExpr::function(FunctionExpr {
            function_name: "is_not_null".to_string(),
            arguments: Some(vec![col]),
        })),
        FilterOperator::Between => {
            let low = parse_scalar_value(&cond.value);
            let high = parse_scalar_value(&cond.value2);
            Some(ParsedExpr {
                between_expr: Some(Box::new(BetweenExpr {
                    value: Box::new(col),
                    lower_bound: Box::new(low),
                    upper_bound: Box::new(high),
                })),
                ..Default::default()
            })
        }
        _ => {
            let func_name = match cond.operator {
                FilterOperator::Equal => "=",
                FilterOperator::NotEqual => "!=",
                FilterOperator::GreaterThan => ">",
                FilterOperator::GreaterOrEqual => ">=",
                FilterOperator::LessThan => "<",
                FilterOperator::LessOrEqual => "<=",
                FilterOperator::Contains => "like",
                FilterOperator::NotContains => "not like",
                _ => "=",
            };
            let val = parse_scalar_value(&cond.value);
            Some(ParsedExpr::function(FunctionExpr {
                function_name: func_name.to_string(),
                arguments: Some(vec![col, val]),
            }))
        }
    }
}

pub fn parse_scalar_value(s: &str) -> ParsedExpr {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        return ParsedExpr::constant(ConstantExpr::null());
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return ParsedExpr::constant(ConstantExpr::bool(true));
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return ParsedExpr::constant(ConstantExpr::bool(false));
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return ParsedExpr::constant(ConstantExpr::int(i));
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return ParsedExpr::constant(ConstantExpr::float(f));
    }
    // Check if stripped quotes
    let unquoted = if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    ParsedExpr::constant(ConstantExpr::string(unquoted))
}

pub fn sql_expr_to_parsed_expr(expr: &SqlExpr) -> ParsedExpr {
    match expr {
        SqlExpr::Identifier(ident) => ParsedExpr::column(ColumnExpr::single(ident.value.clone())),
        SqlExpr::CompoundIdentifier(idents) => {
            let names: Vec<String> = idents.iter().map(|id| id.value.clone()).collect();
            ParsedExpr {
                column_expr: Some(ColumnExpr {
                    column_name: names,
                    star: false,
                }),
                ..Default::default()
            }
        }
        SqlExpr::Value(val_with_span) => match &val_with_span.value {
            Value::Number(num_str, _) => {
                if let Ok(i) = num_str.parse::<i64>() {
                    ParsedExpr::constant(ConstantExpr::int(i))
                } else if let Ok(f) = num_str.parse::<f64>() {
                    ParsedExpr::constant(ConstantExpr::float(f))
                } else {
                    ParsedExpr::constant(ConstantExpr::string(num_str.clone()))
                }
            }
            Value::SingleQuotedString(s) | Value::DoubleQuotedString(s) => {
                ParsedExpr::constant(ConstantExpr::string(s.clone()))
            }
            Value::Boolean(b) => ParsedExpr::constant(ConstantExpr::bool(*b)),
            Value::Null => ParsedExpr::constant(ConstantExpr::null()),
            _ => ParsedExpr::constant(ConstantExpr::string(val_with_span.value.to_string())),
        },
        SqlExpr::UnaryOp { op, expr: inner } => {
            let op_name = format!("{op}");
            let inner_expr = sql_expr_to_parsed_expr(inner);
            ParsedExpr::function(FunctionExpr {
                function_name: op_name,
                arguments: Some(vec![inner_expr]),
            })
        }
        SqlExpr::BinaryOp { left, op, right } => {
            let op_name = match op {
                BinaryOperator::Eq => "=".to_string(),
                BinaryOperator::NotEq => "!=".to_string(),
                BinaryOperator::Gt => ">".to_string(),
                BinaryOperator::GtEq => ">=".to_string(),
                BinaryOperator::Lt => "<".to_string(),
                BinaryOperator::LtEq => "<=".to_string(),
                BinaryOperator::And => "and".to_string(),
                BinaryOperator::Or => "or".to_string(),
                BinaryOperator::Plus => "+".to_string(),
                BinaryOperator::Minus => "-".to_string(),
                BinaryOperator::Multiply => "*".to_string(),
                BinaryOperator::Divide => "/".to_string(),
                _ => format!("{op}"),
            };
            let left_expr = sql_expr_to_parsed_expr(left);
            let right_expr = sql_expr_to_parsed_expr(right);
            ParsedExpr::function(FunctionExpr {
                function_name: op_name,
                arguments: Some(vec![left_expr, right_expr]),
            })
        }
        SqlExpr::Function(func) => {
            let fname = func.name.to_string();
            let mut args = Vec::new();
            if let FunctionArguments::List(arg_list) = &func.args {
                for arg in &arg_list.args {
                    if let FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) = arg {
                        args.push(sql_expr_to_parsed_expr(e));
                    }
                }
            }
            ParsedExpr::function(FunctionExpr {
                function_name: fname,
                arguments: Some(args),
            })
        }
        SqlExpr::Between {
            expr,
            negated,
            low,
            high,
        } => {
            let val = sql_expr_to_parsed_expr(expr);
            let l = sql_expr_to_parsed_expr(low);
            let h = sql_expr_to_parsed_expr(high);
            let bet = ParsedExpr {
                between_expr: Some(Box::new(BetweenExpr {
                    value: Box::new(val),
                    lower_bound: Box::new(l),
                    upper_bound: Box::new(h),
                })),
                ..Default::default()
            };
            if *negated {
                ParsedExpr::function(FunctionExpr {
                    function_name: "not".to_string(),
                    arguments: Some(vec![bet]),
                })
            } else {
                bet
            }
        }
        SqlExpr::IsNull(e) => {
            let inner = sql_expr_to_parsed_expr(e);
            ParsedExpr::function(FunctionExpr {
                function_name: "is_null".to_string(),
                arguments: Some(vec![inner]),
            })
        }
        SqlExpr::IsNotNull(e) => {
            let inner = sql_expr_to_parsed_expr(e);
            ParsedExpr::function(FunctionExpr {
                function_name: "is_not_null".to_string(),
                arguments: Some(vec![inner]),
            })
        }
        _ => ParsedExpr::constant(ConstantExpr::string(expr.to_string())),
    }
}

#[allow(clippy::large_enum_variant)]
pub enum ParsedSqlCommand {
    Select {
        db_name: Option<String>,
        table_name: String,
        select_list: Vec<ParsedExpr>,
        search_expr: Option<SearchExpr>,
        where_expr: Option<ParsedExpr>,
        group_by_list: Option<Vec<ParsedExpr>>,
        order_by_list: Option<Vec<OrderByExpr>>,
        limit_expr: Option<ParsedExpr>,
        offset_expr: Option<ParsedExpr>,
    },
    Insert {
        db_name: Option<String>,
        table_name: String,
        columns: Vec<String>,
        values: Vec<Vec<ParsedExpr>>,
    },
    Update {
        db_name: Option<String>,
        table_name: String,
        assignments: Vec<UpdateExpr>,
        where_expr: Option<ParsedExpr>,
    },
    Delete {
        db_name: Option<String>,
        table_name: String,
        where_expr: Option<ParsedExpr>,
    },
    CreateTable {
        db_name: Option<String>,
        table_name: String,
        columns: Vec<ColumnDef>,
    },
    DropTable {
        db_name: Option<String>,
        table_name: String,
    },
    CreateDatabase {
        db_name: String,
    },
    DropDatabase {
        db_name: String,
    },
    Use {
        db_name: String,
    },
    ShowTables {
        db_name: Option<String>,
    },
    ShowDatabases,
    ShowColumns {
        db_name: Option<String>,
        table_name: String,
    },
    Explain {
        db_name: Option<String>,
        table_name: String,
        select_list: Vec<ParsedExpr>,
        where_expr: Option<ParsedExpr>,
    },
}

pub fn parse_sql(sql: &str) -> Result<ParsedSqlCommand, String> {
    let dialect = GenericDialect {};
    let trimmed = sql.trim().trim_end_matches(';').trim();

    // Fast-path for SHOW/USE commands
    let lower = trimmed.to_ascii_lowercase();
    if lower == "show databases" {
        return Ok(ParsedSqlCommand::ShowDatabases);
    }
    if lower == "show tables" {
        return Ok(ParsedSqlCommand::ShowTables { db_name: None });
    }
    if lower.starts_with("show tables from ") {
        let db = trimmed[17..].trim().trim_matches(|c| c == '`' || c == '"');
        return Ok(ParsedSqlCommand::ShowTables {
            db_name: Some(db.to_string()),
        });
    }
    if lower.starts_with("show columns from ") {
        let target = trimmed[18..].trim().trim_matches(|c| c == '`' || c == '"');
        let parts: Vec<&str> = target.split('.').collect();
        if parts.len() == 2 {
            return Ok(ParsedSqlCommand::ShowColumns {
                db_name: Some(parts[0].to_string()),
                table_name: parts[1].to_string(),
            });
        }
        return Ok(ParsedSqlCommand::ShowColumns {
            db_name: None,
            table_name: target.to_string(),
        });
    }
    if lower.starts_with("use ") {
        let db = trimmed[4..].trim().trim_matches(|c| c == '`' || c == '"');
        return Ok(ParsedSqlCommand::Use {
            db_name: db.to_string(),
        });
    }

    let statements =
        Parser::parse_sql(&dialect, trimmed).map_err(|e| format!("Failed to parse SQL: {e}"))?;

    let statement = statements
        .into_iter()
        .next()
        .ok_or_else(|| "Empty SQL statement".to_string())?;

    match statement {
        Statement::Query(query) => parse_query_statement(*query, false),
        Statement::Explain { statement, .. } => {
            if let Statement::Query(query) = *statement {
                parse_query_statement(*query, true)
            } else {
                Err("EXPLAIN is only supported for SELECT statements".to_string())
            }
        }
        Statement::Insert(insert) => {
            let table_name_full = insert.table.to_string();
            let (db_name, table_name) = split_db_and_table(&table_name_full);
            let columns: Vec<String> = insert.columns.iter().map(|c| c.to_string()).collect();
            let mut all_values = Vec::new();
            if let Some(source) = insert.source
                && let SetExpr::Values(values) = *source.body
            {
                for row in values.rows {
                    let parsed_row: Vec<ParsedExpr> =
                        row.iter().map(sql_expr_to_parsed_expr).collect();
                    all_values.push(parsed_row);
                }
            }
            Ok(ParsedSqlCommand::Insert {
                db_name,
                table_name,
                columns,
                values: all_values,
            })
        }
        Statement::Update(update) => {
            let table_name_full = update.table.relation.to_string();
            let (db_name, table_name) = split_db_and_table(&table_name_full);
            let mut update_assignments = Vec::new();
            for a in update.assignments {
                let col = a.target.to_string();
                let val = sql_expr_to_parsed_expr(&a.value);
                update_assignments.push(UpdateExpr {
                    column_name: col,
                    value: val,
                });
            }
            let where_expr = update.selection.as_ref().map(sql_expr_to_parsed_expr);
            Ok(ParsedSqlCommand::Update {
                db_name,
                table_name,
                assignments: update_assignments,
                where_expr,
            })
        }
        Statement::Delete(delete) => {
            let table_name_full = match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(tables)
                | sqlparser::ast::FromTable::WithoutKeyword(tables) => {
                    tables.first().map(|t| t.to_string()).unwrap_or_default()
                }
            };
            let (db_name, table_name) = split_db_and_table(&table_name_full);
            let where_expr = delete.selection.as_ref().map(sql_expr_to_parsed_expr);
            Ok(ParsedSqlCommand::Delete {
                db_name,
                table_name,
                where_expr,
            })
        }
        Statement::CreateTable(create) => {
            let table_name_full = create.name.to_string();
            let (db_name, table_name) = split_db_and_table(&table_name_full);
            let mut col_defs = Vec::new();
            for (idx, col) in create.columns.iter().enumerate() {
                let dt = map_sql_data_type(&col.data_type);
                let mut constraints = Vec::new();
                for c in &col.options {
                    match &c.option {
                        ColumnOption::PrimaryKey(_) => {
                            constraints.push(ConstraintType::PrimaryKey);
                        }
                        ColumnOption::Unique(_) => {
                            constraints.push(ConstraintType::Unique);
                        }
                        ColumnOption::NotNull => {
                            constraints.push(ConstraintType::NotNull);
                        }
                        ColumnOption::Null => {
                            constraints.push(ConstraintType::Null);
                        }
                        _ => {}
                    }
                }
                col_defs.push(ColumnDef {
                    id: idx as i64,
                    data_type: dt,
                    name: col.name.value.clone(),
                    constraints,
                    constant_expr: None,
                    comment: None,
                });
            }
            Ok(ParsedSqlCommand::CreateTable {
                db_name,
                table_name,
                columns: col_defs,
            })
        }
        Statement::Drop {
            object_type, names, ..
        } => {
            let name = names.first().map(|n| n.to_string()).unwrap_or_default();
            match object_type {
                sqlparser::ast::ObjectType::Table => {
                    let (db_name, table_name) = split_db_and_table(&name);
                    Ok(ParsedSqlCommand::DropTable {
                        db_name,
                        table_name,
                    })
                }
                sqlparser::ast::ObjectType::Database | sqlparser::ast::ObjectType::Schema => {
                    Ok(ParsedSqlCommand::DropDatabase { db_name: name })
                }
                _ => Err(format!("Unsupported DROP object: {object_type}")),
            }
        }
        Statement::CreateDatabase { db_name, .. } => Ok(ParsedSqlCommand::CreateDatabase {
            db_name: db_name.to_string(),
        }),
        Statement::CreateSchema { schema_name, .. } => Ok(ParsedSqlCommand::CreateDatabase {
            db_name: schema_name.to_string(),
        }),
        _ => Err(format!("Unsupported statement type: {statement}")),
    }
}

fn parse_query_statement(query: Query, is_explain: bool) -> Result<ParsedSqlCommand, String> {
    let SetExpr::Select(select) = *query.body else {
        return Err("Only standard SELECT queries are supported".to_string());
    };

    let table_factor = select
        .from
        .first()
        .map(|f| &f.relation)
        .ok_or_else(|| "SELECT without FROM table is not supported".to_string())?;

    let table_name_full = match table_factor {
        TableFactor::Table { name, .. } => name.to_string(),
        _ => return Err("Only base tables are supported in FROM".to_string()),
    };

    let (db_name, table_name) = split_db_and_table(&table_name_full);

    let mut select_list = Vec::new();
    let mut search_dense_list = Vec::new();
    let mut search_text_list = Vec::new();
    let mut search_sparse_list = Vec::new();
    let mut search_tensor_list = Vec::new();

    for item in select.projection {
        match item {
            SelectItem::Wildcard(_) => {
                select_list.push(ParsedExpr {
                    column_expr: Some(ColumnExpr::star()),
                    ..Default::default()
                });
            }
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                // Check if search functions MATCH_DENSE, MATCH_TEXT, etc.
                if let SqlExpr::Function(func) = &e {
                    let fname = func.name.to_string().to_ascii_uppercase();
                    if fname == "MATCH_DENSE"
                        || fname == "MATCH_TEXT"
                        || fname == "MATCH_SPARSE"
                        || fname == "MATCH_TENSOR"
                    {
                        parse_search_function(
                            func,
                            &mut search_dense_list,
                            &mut search_text_list,
                            &mut search_sparse_list,
                            &mut search_tensor_list,
                        );
                        continue;
                    }
                }
                select_list.push(sql_expr_to_parsed_expr(&e));
            }
            _ => {}
        }
    }

    if select_list.is_empty() {
        select_list.push(ParsedExpr {
            column_expr: Some(ColumnExpr::star()),
            ..Default::default()
        });
    }

    let where_expr = select.selection.as_ref().map(sql_expr_to_parsed_expr);

    if is_explain {
        return Ok(ParsedSqlCommand::Explain {
            db_name,
            table_name,
            select_list,
            where_expr,
        });
    }

    let order_by_list = if let Some(order_by) = query.order_by {
        if let OrderByKind::Expressions(exprs) = order_by.kind {
            let mut list = Vec::new();
            for o in exprs {
                let parsed = sql_expr_to_parsed_expr(&o.expr);
                let asc = o.options.asc.unwrap_or(true);
                list.push(OrderByExpr { expr: parsed, asc });
            }
            Some(list)
        } else {
            None
        }
    } else {
        None
    };

    let (limit_expr, offset_expr) = match query.limit_clause {
        Some(LimitClause::LimitOffset { limit, offset, .. }) => {
            let lim = limit.as_ref().map(sql_expr_to_parsed_expr);
            let off = offset.as_ref().map(|o| sql_expr_to_parsed_expr(&o.value));
            (lim, off)
        }
        Some(LimitClause::OffsetCommaLimit { offset, limit }) => {
            let lim = Some(sql_expr_to_parsed_expr(&limit));
            let off = Some(sql_expr_to_parsed_expr(&offset));
            (lim, off)
        }
        None => (None, None),
    };

    let search_expr = if !search_dense_list.is_empty()
        || !search_text_list.is_empty()
        || !search_sparse_list.is_empty()
        || !search_tensor_list.is_empty()
    {
        Some(SearchExpr {
            match_exprs: (!search_text_list.is_empty()).then_some(search_text_list),
            match_dense_exprs: (!search_dense_list.is_empty()).then_some(search_dense_list),
            match_sparse_exprs: (!search_sparse_list.is_empty()).then_some(search_sparse_list),
            match_tensor_exprs: (!search_tensor_list.is_empty()).then_some(search_tensor_list),
            fusion_expr: None,
        })
    } else {
        None
    };

    Ok(ParsedSqlCommand::Select {
        db_name,
        table_name,
        select_list,
        search_expr,
        where_expr,
        group_by_list: None,
        order_by_list,
        limit_expr,
        offset_expr,
    })
}

fn parse_search_function(
    func: &sqlparser::ast::Function,
    dense: &mut Vec<MatchDenseExpr>,
    text: &mut Vec<MatchExpr>,
    _sparse: &mut Vec<MatchSparseExpr>,
    _tensor: &mut Vec<MatchTensorExpr>,
) {
    let fname = func.name.to_string().to_ascii_uppercase();
    let mut args = Vec::new();
    if let FunctionArguments::List(arg_list) = &func.args {
        for arg in &arg_list.args {
            if let FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) = arg {
                args.push(e);
            }
        }
    }

    if fname == "MATCH_DENSE" && args.len() >= 4 {
        // MATCH_DENSE(column, embedding_array_or_bytes, distance_type, topn)
        let col = sql_expr_to_parsed_expr(args[0]);
        let dist = args[2].to_string().trim_matches('\'').to_string();
        let topn = args[3].to_string().parse::<i64>().unwrap_or(10);
        dense.push(MatchDenseExpr {
            column_expr: Box::new(col),
            embedding_data: Vec::new(),
            embedding_data_type: ElementType::ElementFloat32,
            distance_type: dist,
            topn,
            opt_params: None,
        });
    } else if fname == "MATCH_TEXT" && args.len() >= 3 {
        // MATCH_TEXT(fields, matching_text, topn)
        let fields = args[0].to_string().trim_matches('\'').to_string();
        let query = args[1].to_string().trim_matches('\'').to_string();
        let topn = args[2].to_string().parse::<i64>().ok();
        text.push(MatchExpr {
            fields,
            matching_text: query,
            options_text: String::new(),
            opt_params: None,
            topn,
        });
    }
}

fn split_db_and_table(name: &str) -> (Option<String>, String) {
    let clean = name.replace(['`', '"'], "");
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() == 2 {
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, clean)
    }
}

fn map_sql_data_type(dt: &SqlDataType) -> DataType {
    match dt {
        SqlDataType::Boolean | SqlDataType::Bool => DataType::simple(LogicType::Boolean),
        SqlDataType::TinyInt(_) => DataType::simple(LogicType::TinyInt),
        SqlDataType::SmallInt(_) => DataType::simple(LogicType::SmallInt),
        SqlDataType::Int(_) | SqlDataType::Integer(_) => DataType::simple(LogicType::Integer),
        SqlDataType::BigInt(_) => DataType::simple(LogicType::BigInt),
        SqlDataType::Float(_) | SqlDataType::Real => DataType::simple(LogicType::Float),
        SqlDataType::Double(_) | SqlDataType::DoublePrecision => {
            DataType::simple(LogicType::Double)
        }
        SqlDataType::Varchar(len) => {
            let length = len.as_ref().and_then(|l| match l {
                sqlparser::ast::CharacterLength::IntegerLength { length, .. } => {
                    Some(*length as i32)
                }
                _ => None,
            });
            DataType {
                logic_type: LogicType::Varchar,
                length,
                ..Default::default()
            }
        }
        SqlDataType::Text => DataType::simple(LogicType::Varchar),
        SqlDataType::Date => DataType::simple(LogicType::Date),
        SqlDataType::Time(_, _) => DataType::simple(LogicType::Time),
        SqlDataType::Timestamp(_, _) => DataType::simple(LogicType::Timestamp),
        SqlDataType::JSON => DataType::simple(LogicType::Json),
        SqlDataType::Custom(name, args) => {
            let tname = name.to_string().to_ascii_lowercase();
            if tname == "embedding" || tname == "vector" {
                // e.g. embedding(float, 128)
                let dim = args
                    .first()
                    .and_then(|a| a.to_string().parse::<i32>().ok())
                    .unwrap_or(128);
                DataType::embedding(ElementType::ElementFloat32, dim)
            } else {
                DataType::simple(LogicType::Varchar)
            }
        }
        _ => DataType::simple(LogicType::Varchar),
    }
}
