# httpserv-here

Serve any local directory over HTTP with a single command.  
用一条命令把任意本地目录通过 HTTP 暴露出去。

## Features | 功能特点
- Instant static hosting for the current or specified directory.  
  立即托管当前目录或指定目录。
- Automatic `index.html` support plus clean directory listings.  
  自动识别 `index.html`，无首页时提供整洁的目录列表。
- Directory rows include file size and last modified time.  
  列表项展示文件大小与最近修改时间。
- Works as a global install or `npx` one-off.  
  支持全局安装或通过 `npx` 临时使用。

## Installation | 安装

```sh
npm install -g httpserv-here
```

Or run it ad‑hoc without installing globally:  
或者临时使用：

```sh
npx httpserv-here
```

## Usage | 使用方式

```sh
httpserv-here [options] [directory]
```

- `directory`: Directory to share; defaults to the current working directory.  
  `directory`：要共享的目录，默认使用当前工作目录。
- `-d, --dir <path>`: Explicit directory override.  
  `-d, --dir <path>`：显式指定要共享的目录。
- `-p, --port <number>`: Port to listen on (default `8080`).  
  `-p, --port <number>`：设置监听端口（默认 `8080`）。
- `-H, --host <address>`: Host/IP to bind (default `0.0.0.0`).  
  `-H, --host <address>`：指定绑定的主机或 IP（默认 `0.0.0.0`）。
- `-h, --help`: Show help.  
  `-h, --help`：查看帮助信息。
- `-V, --version`: Print version.  
  `-V, --version`：查看版本号。

After startup you’ll see the bound addresses in the terminal. Opening them in a browser displays your static files or a table view of the directory contents.  
启动后终端会打印可访问的地址，浏览器打开即可查看静态文件；若目录中没有 `index.html`，将显示带有文件大小和修改时间的目录表格。

## Development | 开发

```sh
npm install
npm start
```

`npm start` serves the current project directory so you can test quickly with a browser or `curl`.  
`npm start` 会把当前项目目录作为静态资源根目录，方便通过浏览器或 `curl` 快速验证。
