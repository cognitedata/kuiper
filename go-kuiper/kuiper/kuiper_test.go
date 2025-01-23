package kuiper

import (
	"strings"
	"testing"
)

func TestKuiperExpressionNoArgs(t *testing.T) {
	expr, err := NewKuiperExpression("1 + 1", []string{})
	if err != nil {
		t.Fatalf("Failed to create expression: %v", err)
	}
	defer expr.Dispose()

	result, err := expr.Run()
	if err != nil {
		t.Fatalf("Failed to run expression: %v", err)
	}
	if strings.TrimSpace(result) != "2" {
		t.Errorf("Expected result '2', got '%s'", result)
	}

	str := expr.String()
	if strings.TrimSpace(str) != "2" {
		t.Errorf("Expected String() to return '2', got '%s'", str)
	}
}

func TestKuiperCompileErr(t *testing.T) {
	_, err := NewKuiperExpression("\"test\".notafunc()", []string{})
	if err == nil {
		t.Fatal("Expected an error, but got nil")
	}

	kuiperErr, ok := err.(*KuiperException)
	if !ok {
		t.Fatalf("Expected error of type *KuiperException, got %T", err)
	}

	expectedMsg := "Compilation failed: Unrecognized function: notafunc at 7..17"
	if kuiperErr.Message != expectedMsg {
		t.Errorf("Expected error message '%s', got '%s'", expectedMsg, kuiperErr.Message)
	}

	if kuiperErr.Start != 7 {
		t.Errorf("Expected Start to be 7, got %d", kuiperErr.Start)
	}

	if kuiperErr.End != 17 {
		t.Errorf("Expected End to be 17, got %d", kuiperErr.End)
	}
}

func TestKuiperWithInputs(t *testing.T) {
	expr, err := NewKuiperExpression("in1 + in2 + in3", []string{"in1", "in2", "in3"})
	if err != nil {
		t.Fatalf("Failed to create expression: %v", err)
	}
	defer expr.Dispose()

	result, err := expr.Run("1", "2", "3")
	if err != nil {
		t.Fatalf("Failed to run expression: %v", err)
	}
	if strings.TrimSpace(result) != "6" {
		t.Errorf("Expected result '6', got '%s'", result)
	}
}
