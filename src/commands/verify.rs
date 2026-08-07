use crate::cli::VerifyArgs;
use crate::commands::resolve_target;
use anyhow::Result;

pub fn run(args: VerifyArgs) -> Result<()> {
    let target = resolve_target(args.target);
    target.verify(args.host.as_deref())
}
