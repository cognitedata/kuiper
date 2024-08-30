# Kuiper, the Cognite mapping language

This package contains Go bindings for the Kuiper programming language, allowing you to build and call Kuiper expressions from Go code.

The language itself is documented [here](https://docs.cognite.com/cdf/integration/guides/extraction/hosted_extractors/kuiper_concepts).

## Installation

To use this library in your Go project, run:

```
go get github.com/cognitedata/go-kuiper
```

Make sure you have the appropriate Rust library files (`.so`, `.dll`, or `.dylib`) in your system's library path or in the same directory as your Go executable.

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