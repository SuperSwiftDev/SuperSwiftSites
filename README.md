# Example

```
➜ ./scripts/ssio.sh compile --template sample/base.html --input sample/pages/*.html --output sample/output
```

```
➜ tree sample
sample
├── base.css
├── base.html
├── logo.png
├── main.html
├── navigation.html
├── output
│   ├── index.html
│   ├── page1.html
│   ├── page2.html
│   └── page3.html
└── pages
    ├── index.html
    ├── page1.html
    ├── page2.html
    └── page3.html
```

# Overview

The biggest innovation so far is better reusability.

Consider the following files,

**`sample/pages/index.html`:**
```html
<include src="../main.html">
    <h1>Homepage - Hello World!</h1>
</include>
```

**`sample/main.html`:**
```html
<include src="navigation.html"></include>
<main>
    <content></content>
</main>
```

**`sample/navigation.html`:**
```html
<nav>
    <nav-link from="pages/index.html" as="index.html">
        <img src="logo.png" alt="Your Company Name">
    </nav-link>
    <ol>
        <li><nav-link from="pages/page1.html" as="page1.html">Page 1</nav-link></li>
        <li><nav-link from="pages/page2.html" as="page2.html">Page 2</nav-link></li>
        <li><nav-link from="pages/page3.html" as="page3.html">Page 3</nav-link></li>
    </ol>
</nav>
```


Furthermore invoking `ssio` with `--template sample/base.html` will implicitly wrap all `--input sample/pages/*.html` page contents with the following template as defined in `sample/base.html`:
```html
<!DOCTYPE html>
<html>
<head>
    <title>My Site</title>
    <link
        href="https://fonts.googleapis.com/css?family=Source+Sans+Pro:200,200i,300,300i,400,400i,600,600i,700,700i,900,900i&display=swap&subset=latin-ext"
        rel="stylesheet">
    <link rel="stylesheet" href="https://use.fontawesome.com/releases/v5.8.2/css/all.css"
        integrity="sha384-oS3vJWv+0UjzBfQzYUhtDYW+Pj2yciDJxpsK1OYPAYjqT085Qq/1cq5FLXAZQ7Ay" crossorigin="anonymous">
    <meta http-equiv="Content-type" content="text/html; charset=utf-8" />
    <style>
        html, body {
            margin: 0;
            padding: 0;
            height: 100%;
            box-sizing: border-box;
        }
    </style>
    <link rel="stylesheet" href="base.css">
</head>
<body>
    <content></content>
</body>
</html>
```