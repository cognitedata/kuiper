package kuiper

/*
#cgo LDFLAGS: -L${SRCDIR}/lib -lkuiper_interop
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct {
    const char* error;
    bool is_error;
    uint64_t start;
    uint64_t end;
} KuiperError;

typedef struct {
    KuiperError error;
    void* result;
} CompileResult;

typedef struct {
    KuiperError error;
    const char* result;
} TransformResult;

extern CompileResult* compile_expression(const char* data, const char** inputs, size_t len);
extern void* get_expression_from_compile_result(CompileResult* result);
extern TransformResult* run_expression(const char** data, size_t len, void* expression);
extern const char* expression_to_string(void* expression);
extern void destroy_expression(void* expression);
*/
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

type KuiperException struct {
	Message string
	Start   uint64
	End     uint64
}

func (e *KuiperException) Error() string {
	return e.Message
}

type KuiperExpression struct {
	ptr unsafe.Pointer
}

func NewKuiperExpression(expression string, inputs []string) (*KuiperExpression, error) {
	cExpr := C.CString(expression)
	defer C.free(unsafe.Pointer(cExpr))

	cInputs := make([]*C.char, len(inputs))
	for i, input := range inputs {
		cInputs[i] = C.CString(input)
		defer C.free(unsafe.Pointer(cInputs[i]))
	}

	var result *C.CompileResult
	if len(inputs) > 0 {
		result = C.compile_expression(cExpr, (**C.char)(unsafe.Pointer(&cInputs[0])), C.size_t(len(inputs)))
	} else {
		result = C.compile_expression(cExpr, nil, 0)
	}

	if result == nil {
		return nil, fmt.Errorf("failed to compile expression")
	}

	if result.error.is_error {
		return nil, &KuiperException{
			Message: C.GoString(result.error.error),
			Start:   uint64(result.error.start),
			End:     uint64(result.error.end),
		}
	}

	expr := C.get_expression_from_compile_result(result)
	if expr == nil {
		return nil, fmt.Errorf("failed to get expression from compile result")
	}

	return &KuiperExpression{ptr: expr}, nil
}

func (ke *KuiperExpression) Run(inputs ...string) (string, error) {
	if ke.ptr == nil {
		return "", errors.New("expression is nil")
	}

	var cInputs **C.char
	var inputsLen C.size_t

	if len(inputs) > 0 {
		cInputsSlice := make([]*C.char, len(inputs))
		for i, input := range inputs {
			cInputsSlice[i] = C.CString(input)
			defer C.free(unsafe.Pointer(cInputsSlice[i]))
		}
		cInputs = (**C.char)(unsafe.Pointer(&cInputsSlice[0]))
		inputsLen = C.size_t(len(inputs))
	}

	result := C.run_expression(cInputs, inputsLen, ke.ptr)
	if result == nil {
		return "", errors.New("failed to run expression")
	}

	if result.error.is_error {
		err := errors.New(C.GoString(result.error.error))
		return "", err
	}

	return C.GoString(result.result), nil
}

func (ke *KuiperExpression) String() string {
	if ke.ptr == nil {
		return ""
	}
	return C.GoString(C.expression_to_string(ke.ptr))
}

func (ke *KuiperExpression) Dispose() {
	if ke.ptr != nil {
		C.destroy_expression(ke.ptr)
		ke.ptr = nil
	}
}
