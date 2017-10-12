# php-stacktrace

Sampling profiler for PHP programs, inspired by [ruby-stacktrace](https://github.com/jvns/ruby-stacktrace).

# Install

1. [Download](https://github.com/oraoto/php-stacktrace/releases) or build `php-stacktrace`
1. Install debuginfo for php, `sudo dnf debuginfo-install php-debuginfo`
1. Find debuginfo path, eg, `/usr/lib/debug/.dwz/php71u-7.1.9-2.ius.centos7.x86_64`

# Usage

## Get stacktrace with DWARF debuginfo

~~~
./php-stacktrace -d <path to debuginfo> <pid>
~~~

## Write config

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
./php-stacktrace -c <path to config> -w <path to config> <pid>
~~~

For a running Laravel applicaiton, the output looks like:

~~~
sleep()
Illuminate\Routing\Router->{closure}()
Illuminate\Routing\Route->runCallable()
Illuminate\Routing\Route->run()
Illuminate\Routing\Router->Illuminate\Routing\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Routing\Middleware\SubstituteBindings->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Foundation\Http\Middleware\VerifyCsrfToken->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\View\Middleware\ShareErrorsFromSession->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Session\Middleware\StartSession->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Cookie\Middleware\AddQueuedCookiesToResponse->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Cookie\Middleware\EncryptCookies->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Pipeline\Pipeline->then()
Illuminate\Routing\Router->runRouteWithinStack()
Illuminate\Routing\Router->dispatchToRoute()
Illuminate\Routing\Router->dispatch()
Illuminate\Foundation\Http\Kernel->Illuminate\Foundation\Http\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Foundation\Http\Middleware\TransformsRequest->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Foundation\Http\Middleware\TransformsRequest->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Foundation\Http\Middleware\ValidatePostSize->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Foundation\Http\Middleware\CheckForMaintenanceMode->handle()
Illuminate\Pipeline\Pipeline->Illuminate\Pipeline\{closure}()
Illuminate\Routing\Pipeline->Illuminate\Routing\{closure}()
Illuminate\Pipeline\Pipeline->then()
Illuminate\Foundation\Http\Kernel->sendRequestThroughRouter()
Illuminate\Foundation\Http\Kernel->handle()
~~~