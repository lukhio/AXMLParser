# AXMLParser

Every APK has a manifest file, which is usually in binary format. This project
decodes this manifest into human-readable XML.

### Usage

```
./AXMLParser [AXML|APK]
```

The argument can be either the manifest directly (in binary format) or an APK
file, in which case the manifest will first be extracted from the APK.

### To do

- when extracting from an APK, also decode other resources (e.g.,
  `strings.xml`) which would allow us to resolve some static references.
- when printing decoded XML to `stdout`, pretty-print it instead of just
  dumping it on one line.
