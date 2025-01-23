# Kuiper, the Cognite mapping language

This package contains Go bindings for the Kuiper programming language, allowing you to build and call Kuiper expressions from Go code.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

## Requirements

- Go 1.20 or later
- GCC or compatible C compiler (for CGo)

## Installation

To use this library in your Go project, run:

```bash
go get github.com/cognitedata/go-kuiper
```

The package includes pre-compiled shared libraries for various platforms. Make sure you're using a supported platform:
- Linux (amd64) - to be added
- macOS (amd64, arm64) - to be added for amd64
- Windows (amd64) - to be added

## Usage

Here's a simple example of how to use the Kuiper Go bindings:

```go
package main

import (
	"fmt"
	"log"

	"github.com/cognitedata/go-kuiper/kuiper"
)

func main() {
	// Create a new Kuiper expression
	expression := "in1 + in2.test"
	fmt.Printf("Creating Kuiper expression: %s\n", expression)
	expr, err := kuiper.NewKuiperExpression(expression, []string{"in1", "in2"})
	if err != nil {
		log.Fatalf("Failed to create expression: %v", err)
	}
	defer expr.Dispose()

	// Apply the expression to some data
	data1 := "1"
	data2 := `{"test": 2}`
	fmt.Printf("Running Kuiper expression with data: %s, %s\n", data1, data2)
	result, err := expr.Run(data1, data2)
	if err != nil {
		log.Fatalf("Failed to run expression: %v", err)
	}

	fmt.Printf("Result: %s\n", result)
}
```

This example creates a Kuiper expression that adds two inputs, then runs it with the inputs `"1"` and `{"test": 2}`.

## Running Tests

To run the tests for this package, follow these steps:

1. Navigate to the `go-kuiper` directory in your terminal.

2. Run the following command:

   ```bash
   go test ./kuiper -v
   ```

   This will run all tests in the `kuiper` package with verbose output.

3. To run a specific test, use the `-run` flag followed by the test name:

   ```bash
   go test ./kuiper -v -run TestKuiperExpressionNoArgs
   ```
