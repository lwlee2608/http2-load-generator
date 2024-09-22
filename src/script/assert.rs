use crate::error::Error;
use crate::script::Script;
use crate::script::ScriptContext;
use crate::script::ScriptVariable;
use crate::script::Value;

pub enum AssertOperator {
    Equal,
    NotEqual,
}

pub struct AssertScript {
    pub lhs: ScriptVariable,
    pub rhs: ScriptVariable,
    pub operator: AssertOperator,
}

impl Script for AssertScript {
    fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error> {
        let lhs: Value = self.lhs.get_value(ctx)?;
        let rhs: Value = self.rhs.get_value(ctx)?;

        match self.operator {
            AssertOperator::Equal => assert_equal(lhs, rhs),
            AssertOperator::NotEqual => assert_not_equal(lhs, rhs),
        }
    }
}

fn assert_equal(lhs: Value, rhs: Value) -> Result<(), Error> {
    if lhs != rhs {
        return Err(Error::AssertError(
            format!("assert equal failed: {} != {}", lhs, rhs).into(),
        ));
    }

    Ok(())
}

fn assert_not_equal(lhs: Value, rhs: Value) -> Result<(), Error> {
    if lhs == rhs {
        return Err(Error::AssertError(
            format!("assert not equal failed: {} == {}", lhs, rhs).into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Global;
    use crate::script::ScriptContext;
    use crate::script::ScriptVariable;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_script_assert_equal_variables_success() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut ctx = ScriptContext::new(Arc::clone(&global));

        ctx.set_variable("a", Value::Int(1));
        ctx.set_variable("b", Value::Int(1));

        let script = AssertScript {
            lhs: ScriptVariable::Variable("a".into()),
            rhs: ScriptVariable::Variable("b".into()),
            operator: AssertOperator::Equal,
        };

        let result = script.execute(&mut ctx).unwrap();

        assert_eq!(result, ());
    }

    #[test]
    fn test_script_assert_equal_variables_fail() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut ctx = ScriptContext::new(Arc::clone(&global));

        ctx.set_variable("a", Value::Int(1));
        ctx.set_variable("b", Value::Int(2));

        let script = AssertScript {
            lhs: ScriptVariable::Variable("a".into()),
            rhs: ScriptVariable::Variable("b".into()),
            operator: AssertOperator::Equal,
        };

        let result = script.execute(&mut ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Assert error: assert equal failed: 1 != 2");
    }

    #[test]
    fn test_script_assert_equal_constant_success() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("responseStatus", Value::Int(200));

        let script = AssertScript {
            lhs: ScriptVariable::Variable("responseStatus".into()),
            rhs: ScriptVariable::Constant(Value::Int(200)),
            operator: AssertOperator::Equal,
        };

        let result = script.execute(&mut ctx).unwrap();

        assert_eq!(result, ());
    }

    #[test]
    fn test_script_assert_equal_constant_fail() {
        let global = Global::empty();
        let global = Arc::new(RwLock::new(global));
        let mut ctx = ScriptContext::new(Arc::clone(&global));
        ctx.set_variable("responseStatus", Value::Int(200));

        let script = AssertScript {
            lhs: ScriptVariable::Variable("responseStatus".into()),
            rhs: ScriptVariable::Constant(Value::Int(201)),
            operator: AssertOperator::Equal,
        };

        let result = script.execute(&mut ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(
            err.to_string(),
            "Assert error: assert equal failed: 200 != 201"
        );
    }
}
