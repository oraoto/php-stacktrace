# php-stacktrace

Read stacktrace from outside PHP process, inspired by [ruby-stacktrace](https://github.com/jvns/ruby-stacktrace).

# Install

1. [Download](https://github.com/oraoto/php-stacktrace/releases) or build `php-stacktrace` from source

## Build from source

Clone and build this project:

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
stream_select()
Symfony\Component\Process\Pipes\UnixPipes::readAndWrite()
Symfony\Component\Process\Process::readPipes()
Symfony\Component\Process\Process::wait()
Symfony\Component\Process\Process::run()
Illuminate\Queue\Listener::runProcess()
Illuminate\Queue\Listener::listen()
Illuminate\Queue\Console\ListenCommand::handle()
call_user_func_array()
Illuminate\Container\BoundMethod::Illuminate\Container\{closure}()
Illuminate\Container\Util::unwrapIfClosure()
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
Laravel\Lumen\Console\Kernel::handle()
main()
Time 124.716µs
~~~
