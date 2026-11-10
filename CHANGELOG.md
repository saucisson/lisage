# Changelog

## [0.2.0](https://github.com/saucisson/lisage/compare/v0.1.0...v0.2.0) (2026-11-10)


### Features

* actively wait for ci ([a6eb74a](https://github.com/saucisson/lisage/commit/a6eb74a8e60f1090916946572a2cabc2f4c27ed3))
* add script to synchronize daily from main ([8401400](https://github.com/saucisson/lisage/commit/8401400f9a3282b53c0e73103b0057504948ab9c))
* agent-written worklog audited at the daily merge ([d6379b4](https://github.com/saucisson/lisage/commit/d6379b4d9000a1bd5527e1831295b2393d534320))
* initial commit ([5b04128](https://github.com/saucisson/lisage/commit/5b04128e0126225cee490f3c54bd74d9f6e2c14e))
* milestone filter for the night loop ([0030b3e](https://github.com/saucisson/lisage/commit/0030b3e86f73469133c772420265d61d79209b2b))
* more output during night session ([9e8dcce](https://github.com/saucisson/lisage/commit/9e8dcce1d88d6e092bbe8c70320ec93afb12aa6d))
* night autonomy trace and PR body updates ([d43d8a6](https://github.com/saucisson/lisage/commit/d43d8a6e81c0a28bf8ec8ee82d843490a39d176b))
* one session context per pull request ([b4c32c0](https://github.com/saucisson/lisage/commit/b4c32c0c33e98b1128acbf8d835e3335c494976d))
* recover PRs during autonomous work ([47eadf4](https://github.com/saucisson/lisage/commit/47eadf443dcc45a38abc94ae9961b4aa904f488c))
* unattended night loop ([3463aea](https://github.com/saucisson/lisage/commit/3463aead9e6b6065f162170783549affc309281c))


### Bug Fixes

* align tooling on ruff 0.17 and mypy 2.4, pass gitleaks PR token ([9181e72](https://github.com/saucisson/lisage/commit/9181e72238b44216b59e55497ce27b1f420d8945))
* correct structlog import paths, explicit Any usage and pip-audit export ([c0991f7](https://github.com/saucisson/lisage/commit/c0991f7da0e30d78c21641955b9414b10b91ef69))
* force root handler installation in configure_logging ([aceb240](https://github.com/saucisson/lisage/commit/aceb240dce67c0257e4fe7515a3f926e2d5350e0))
* install uv in the security job for pip-audit ([a223f09](https://github.com/saucisson/lisage/commit/a223f091465952980edcb278dfc4323718550713))
* load documented uppercase env vars, type the webhook as SecretStr in tests ([d446762](https://github.com/saucisson/lisage/commit/d446762e0787a2349ec70353c2c32d9a21a91faa))
* make the review the last gate, negotiate the merge method ([53b7eb7](https://github.com/saucisson/lisage/commit/53b7eb752040e36a72173ba9c447b823450d8e4a))
* parameterize _SubParsersAction and drop explicit Any in dispatcher ([2aa4d27](https://github.com/saucisson/lisage/commit/2aa4d27a160b953104806dad426cfec11146fdb2))
* repair notify example imports, lockfile wording and mutation gate ([a4a6d54](https://github.com/saucisson/lisage/commit/a4a6d54c83cbbb7c9cbe85818c08494e544806f9))
* restore typed runner call in dispatcher ([39b1589](https://github.com/saucisson/lisage/commit/39b158960027ddf01055dccdc262ee1835287778))
* run night sessions with --output text ([02f59c2](https://github.com/saucisson/lisage/commit/02f59c215b29c2f601600cb96b5e0dbd26197362))


### Documentation

* add initial lisage project documentation ([9d988dc](https://github.com/saucisson/lisage/commit/9d988dc7a51a79ac0b540d81b24e95189b351e48))
* ask for Closes #n in the pull request template ([e7a78a7](https://github.com/saucisson/lisage/commit/e7a78a740f5437337494caa9f05064b050b71935))
