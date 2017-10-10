# php-stacktrace

Sampling profiler for PHP programs, inspired by [ruby-stacktrace](https://github.com/jvns/ruby-stacktrace).

# Usage

1. [Download](https://github.com/oraoto/php-stacktrace/releases) or build `php-stacktrace`
1. Install debuginfo for php, eg `sudo dnf debuginfo-install php-debuginfo`
1. Find debuginfo path, eg `/usr/lib/debug/.dwz/php71u-7.1.9-2.ius.centos7.x86_64`
1. ./php-stacktrace trace <debuginfo> <pid>
