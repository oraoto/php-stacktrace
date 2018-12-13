# php-stacktrace

Read stacktrace from outside PHP process, inspired by [ruby-stacktrace](https://github.com/jvns/ruby-stacktrace).

# Install

1. [Download](https://github.com/oraoto/php-stacktrace/releases) or build `php-stacktrace`

# Build from source

You need a latest stable version of Rust to compile. With `rustup`, you can:

```
rustup default stable && rustup update
```

To clone and build this project:

```
git clone https://github.com/oraoto/php-stacktrace.git
cd php-stacktrace
cargo build
```

# Usage

```
./php-stacktrace --help
php-stacktrace 0.2.0
Read stacktrace from outside PHP process

USAGE:
    php-stacktrace [OPTIONS] <PID>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -v <php_version>        PHP Version (5.6, 7.2, 7.3) [default: 7.3]

ARGS:
    <PID>    PID of the PHP process

```

For a running Laravel queue worker, the output looks like:

~~~
sleep()
Illuminate\Queue\Worker::sleep()
Illuminate\Queue\Worker::daemon()
Illuminate\Queue\Console\WorkCommand::runWorker()
Illuminate\Queue\Console\WorkCommand::handle()
call_user_func_array()
Illuminate\Container\BoundMethod::Illuminate\Container\{closure}()
Illuminate\Container\BoundMethod::callBoundMethod()
Illuminate\Container\BoundMethod::call()
Illuminate\Container\Container::call()
Illuminate\Console\Command::execute()
Symfony\Component\Console\Command\Command::run()
Illuminate\Console\Command::run()
Symfony\Component\Console\Application::doRunCommand()
Symfony\Component\Console\Application::doRun()
Symfony\Component\Console\Application::run()
Illuminate\Console\Application::run()
Illuminate\Foundation\Console\Kernel::handle()
main()
Time 168.07µs
~~~
