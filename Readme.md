# php-stacktrace
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Foraoto%2Fphp-stacktrace.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Foraoto%2Fphp-stacktrace?ref=badge_shield)


Read stacktrace from outside PHP process, inspired by [ruby-stacktrace](https://github.com/jvns/ruby-stacktrace).

# Install

1. [Download](https://github.com/oraoto/php-stacktrace/releases) or build `php-stacktrace`
1. Install debuginfo for php, `sudo dnf debuginfo-install php-debuginfo`
1. Find debuginfo path, eg, `/usr/lib/debug/.dwz/php71u-7.1.9-2.ius.centos7.x86_64`

# Build from source

You need a nightly version of Rust to compile. With `rustup`, you can:

```
rustup default nightly && rustup update
```

To clone and build this project:

```
git clone https://github.com/oraoto/php-stacktrace.git
cd php-stacktrace
cargo build
```

# Usage

```
$ ./php-stacktrace --help
php-stacktrace 0.1.2
Read stacktrace from outside PHP process

USAGE:
    php-stacktrace [FLAGS] [OPTIONS] <PID>

FLAGS:
    -a               Attch to process
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c <config_path>              Path to config file, conflicts with -d, -w
    -d <debuginfo_path>           Path to php debuginfo
    -w <write_config_path>        Write config to file, requires -d

ARGS:
    <PID>    PID of the PHP process
```

## Get stacktrace with DWARF debuginfo

~~~
./php-stacktrace -d <path to debuginfo> <pid>
~~~

## Generate config

~~~
./php-stacktrace -d <path to debuginfo> -w <path to config> <pid>
~~~

The config file contains basic info of PHP struct and unions:

~~~
{
  "eg_byte_size": 1552,
  "eg_current_execute_data_offset": 480,
  "eg_vm_stack_top_offset": 456,
  "eg_vm_stack_end_offset": 464,
  "eg_vm_stack_offset": 472,
  "ed_byte_size": 80,
  "ed_this_offset": 32,
  "ed_func_offset": 24,
  "ed_prev_execute_data_offset": 48,
  "fu_function_name_offset": 8,
  "fu_scope_offset": 16,
  "zend_string_len_offset": 16,
  "zend_string_val_offset": 24,
  "stack_byte_size": 24,
  "stack_end_offset": 8,
  "ce_name_offset": 8
}
~~~


## Get stacktrace with config (faster)

~~~
./php-stacktrace -c <path to config> <pid>
~~~

For a running Laravel queue worker, the output looks like:

~~~
sleep()
Illuminate\Queue\Worker->sleep()
Illuminate\Queue\Worker->daemon()
Illuminate\Queue\Console\WorkCommand->runWorker()
Illuminate\Queue\Console\WorkCommand->handle()
call_user_func_array()
Illuminate\Container\BoundMethod->Illuminate\Container\{closure}()
Illuminate\Container\BoundMethod->callBoundMethod()
Illuminate\Container\BoundMethod->call()
Illuminate\Container\Container->call()
Illuminate\Console\Command->execute()
Symfony\Component\Console\Command\Command->run()
Illuminate\Console\Command->run()
Symfony\Component\Console\Application->doRunCommand()
Symfony\Component\Console\Application->doRun()
Symfony\Component\Console\Application->run()
Illuminate\Console\Application->run()
Illuminate\Foundation\Console\Kernel->handle()
main()
~~~


## License
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Foraoto%2Fphp-stacktrace.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Foraoto%2Fphp-stacktrace?ref=badge_large)