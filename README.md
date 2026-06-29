`depthmesh` is a cli tool that re-generates a mesh from a depth image.

## Build

```shell
git clone https://github.com/levinion/depthmesh
cd depthmesh
cargo install --path .
```

## Usage

```txt
Usage: depthmesh [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>
  -o, --output <OUTPUT>          [default: mesh.obj]
  -n, --normal <NORMAL>
  -m, --mask <MASK>
  -t, --threshold <THRESHOLD>    [default: 0]
  -s, --scale <SCALE>            [default: 1]
  -f, --fov <FOV>
      --fx <FX>
      --fy <FY>
      --cx <CX>
      --cy <CY>
      --optimize
      --reduction <REDUCTION>    [default: 0.1]
      --error <ERROR>            [default: 0.01]
      --smooth
      --normalize
      --lambda <LAMBDA>          [default: 0.1]
      --iterations <ITERATIONS>  [default: 10]
  -h, --help                     Print help
  -V, --version                  Print version
```

### Example

```shell
depthmesh -i depth.exr -n normal.exr -f 90 -t 0.1 -o mesh.obj --optimize --smooth
```

### Issue

- If input image contains non-RGB channel such as 'Z' or 'Y', should convert it to 'R' instead.
