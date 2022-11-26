#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use std::process::Command;

    #[test]
    fn requires_arguments() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("athena")?;
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage:"));
        
        Ok(())
    }

    #[test]
    fn rejects_invalid_arguments() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("athena")?;
        cmd.arg("-i").arg("file/that/doesnt/exist");
        cmd.arg("-o").arg("dir/that/doesnt/exist");
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Specified file or directory does not exist"));

        Ok(())
    }

    #[test]
    fn prompts_on_output_invalid() -> Result<(), Box<dyn std::error::Error>> {
      let mut cmd = Command::cargo_bin("athena")?;
      cmd.arg("-i").arg("./");
      cmd.arg("-o").arg("./local/dir/that/doesnt/exist");
      
      // Assert that the user is prompted for whether to create the output directory,
      // and that the command fails if the user enters 'n'
      cmd.assert()
          .failure()
          .stderr(predicate::str::contains("Output directory does not exist: './local/dir/that/doesnt/exist'"))
          .stderr(predicate::str::contains("Create it? [y/N]"))
          .stderr(predicate::str::contains("Output directory does not exist"));

      // Assert that the command succeeds if the user enters 'y' after being prompted
      let mut cmd = Command::cargo_bin("athena")?;
      cmd.arg("-i").arg("./");
      cmd.arg("-o").arg("./local/dir/that/doesnt/exist");

      Ok(())
    }
}
