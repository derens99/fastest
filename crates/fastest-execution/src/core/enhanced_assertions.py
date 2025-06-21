"""Enhanced Assertion Introspection for Fastest

This module provides advanced assertion introspection that matches or exceeds
pytest's capabilities, including:
- Complex expression evaluation
- Detailed comparison formatting
- Collection diffs
- Custom assertion messages
- Chained comparisons
"""

import ast
import inspect
import traceback
import difflib
import pprint
from typing import Any, Dict, List, Optional, Tuple


class AssertionIntrospector:
    """Advanced assertion introspection with pytest-level detail"""
    
    def __init__(self):
        self.max_repr_length = 80
        self.max_diff_lines = 50
        
    def introspect_assertion(self, exc: AssertionError, func: Any, kwargs: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Extract detailed assertion information"""
        tb = exc.__traceback__
        
        # Find the assertion frame
        assertion_frame = self._find_assertion_frame(tb, func.__name__)
        if not assertion_frame:
            return None
            
        # Get assertion source
        assertion_info = self._extract_assertion_source(func, assertion_frame)
        if not assertion_info:
            return None
            
        # Parse and evaluate assertion
        details = self._analyze_assertion(
            assertion_info['source'],
            assertion_frame.tb_frame,
            kwargs
        )
        
        details.update({
            'line': assertion_frame.tb_lineno,
            'function': func.__name__,
            'original_source': assertion_info['source']
        })
        
        return details
    
    def _find_assertion_frame(self, tb, func_name: str):
        """Find the traceback frame containing the assertion"""
        while tb:
            if tb.tb_frame.f_code.co_name == func_name:
                return tb
            tb = tb.tb_next
        return None
    
    def _extract_assertion_source(self, func: Any, tb) -> Optional[Dict[str, str]]:
        """Extract the assertion source code"""
        try:
            source_lines, start_line = inspect.getsourcelines(func)
            line_no = tb.tb_lineno - start_line
            
            if 0 <= line_no < len(source_lines):
                # Handle multi-line assertions
                assertion_lines = []
                i = line_no
                
                # Find the complete assertion statement
                while i < len(source_lines):
                    line = source_lines[i].rstrip()
                    assertion_lines.append(line)
                    
                    # Check if line continues
                    if not line.endswith('\\') and self._is_complete_statement(''.join(assertion_lines)):
                        break
                    i += 1
                
                full_assertion = ' '.join(line.strip() for line in assertion_lines)
                
                # Extract just the assertion part
                if 'assert ' in full_assertion:
                    start = full_assertion.find('assert ') + 7
                    comma_pos = full_assertion.find(',', start)
                    
                    # Handle custom messages
                    if comma_pos > 0 and self._is_assertion_message_comma(full_assertion, comma_pos):
                        assertion_code = full_assertion[start:comma_pos].strip()
                        message = full_assertion[comma_pos+1:].strip().strip('"\'')
                        return {
                            'source': assertion_code,
                            'message': message
                        }
                    else:
                        return {
                            'source': full_assertion[start:].strip(),
                            'message': None
                        }
                        
        except Exception:
            pass
        return None
    
    def _is_complete_statement(self, code: str) -> bool:
        """Check if code is a complete Python statement"""
        try:
            ast.parse(code)
            return True
        except SyntaxError:
            return False
    
    def _is_assertion_message_comma(self, code: str, comma_pos: int) -> bool:
        """Check if comma separates assertion from message"""
        try:
            # Try parsing assertion part only
            assertion_part = code[:comma_pos].replace('assert ', '')
            ast.parse(assertion_part, mode='eval')
            return True
        except:
            return False
    
    def _analyze_assertion(self, assertion_code: str, frame, kwargs: Dict[str, Any]) -> Dict[str, Any]:
        """Analyze the assertion and extract values"""
        details = {
            'assertion': assertion_code,
            'values': {},
            'type': 'unknown'
        }
        
        try:
            tree = ast.parse(assertion_code, mode='eval')
            
            # Handle different assertion types
            if isinstance(tree.body, ast.Compare):
                self._analyze_comparison(tree.body, assertion_code, frame, details)
            elif isinstance(tree.body, ast.BoolOp):
                self._analyze_bool_op(tree.body, assertion_code, frame, details)
            elif isinstance(tree.body, ast.UnaryOp) and isinstance(tree.body.op, ast.Not):
                self._analyze_not(tree.body, assertion_code, frame, details)
            elif isinstance(tree.body, ast.Call):
                self._analyze_call(tree.body, assertion_code, frame, details)
            else:
                # Simple boolean expression
                self._evaluate_expression(assertion_code, frame, details)
                
        except Exception:
            # Fallback: extract any identifiers
            self._extract_identifiers(assertion_code, frame, details)
        
        # Add local variables (excluding function args)
        locals_dict = {k: v for k, v in frame.f_locals.items() 
                      if k not in kwargs and not k.startswith('_')}
        if locals_dict:
            details['locals'] = {k: self._safe_repr(v) for k, v in locals_dict.items()}
            
        return details
    
    def _analyze_comparison(self, node: ast.Compare, source: str, frame, details: Dict):
        """Analyze comparison expressions"""
        details['type'] = 'comparison'
        
        # Get left operand
        left_src = ast.get_source_segment(source, node.left) or self._unparse(node.left)
        left_val = self._safe_eval(node.left, frame)
        details['values'][left_src] = self._safe_repr(left_val)
        
        # Get operators and comparators
        ops = []
        for i, (op, comp) in enumerate(zip(node.ops, node.comparators)):
            right_src = ast.get_source_segment(source, comp) or self._unparse(comp)
            right_val = self._safe_eval(comp, frame)
            details['values'][right_src] = self._safe_repr(right_val)
            
            op_str = self._op_to_str(op)
            ops.append({
                'operator': op_str,
                'left': left_src if i == 0 else right_src,
                'right': right_src,
                'result': self._compare_values(left_val if i == 0 else right_val, op, right_val)
            })
            
        details['comparisons'] = ops
        
        # Special handling for equality of collections
        if len(ops) == 1 and isinstance(ops[0]['operator'], str) and ops[0]['operator'] == '==':
            self._add_diff_if_needed(left_val, right_val, details)
    
    def _analyze_bool_op(self, node: ast.BoolOp, source: str, frame, details: Dict):
        """Analyze boolean operations (and/or)"""
        details['type'] = 'bool_op'
        details['operator'] = 'and' if isinstance(node.op, ast.And) else 'or'
        
        values = []
        for value in node.values:
            val_src = ast.get_source_segment(source, value) or self._unparse(value)
            val_result = self._safe_eval(value, frame)
            details['values'][val_src] = self._safe_repr(val_result)
            values.append({
                'expression': val_src,
                'value': val_result,
                'is_truthy': bool(val_result)
            })
            
        details['operands'] = values
    
    def _analyze_not(self, node: ast.UnaryOp, source: str, frame, details: Dict):
        """Analyze not expressions"""
        details['type'] = 'not'
        
        operand_src = ast.get_source_segment(source, node.operand) or self._unparse(node.operand)
        operand_val = self._safe_eval(node.operand, frame)
        
        details['values'][operand_src] = self._safe_repr(operand_val)
        details['operand'] = {
            'expression': operand_src,
            'value': operand_val,
            'is_truthy': bool(operand_val)
        }
    
    def _analyze_call(self, node: ast.Call, source: str, frame, details: Dict):
        """Analyze function calls in assertions"""
        details['type'] = 'call'
        
        # Get function name
        if isinstance(node.func, ast.Name):
            func_name = node.func.id
        elif isinstance(node.func, ast.Attribute):
            func_name = self._unparse(node.func)
        else:
            func_name = 'unknown'
            
        details['function'] = func_name
        
        # Evaluate the call
        call_src = ast.get_source_segment(source, node) or self._unparse(node)
        call_result = self._safe_eval(node, frame)
        details['values'][call_src] = self._safe_repr(call_result)
        details['result'] = call_result
        
        # Get argument values
        args = []
        for arg in node.args:
            arg_src = ast.get_source_segment(source, arg) or self._unparse(arg)
            arg_val = self._safe_eval(arg, frame)
            args.append({
                'expression': arg_src,
                'value': self._safe_repr(arg_val)
            })
        details['arguments'] = args
    
    def _evaluate_expression(self, expr: str, frame, details: Dict):
        """Evaluate a simple expression"""
        details['type'] = 'expression'
        try:
            result = eval(expr, frame.f_globals, frame.f_locals)
            details['values'][expr] = self._safe_repr(result)
            details['result'] = bool(result)
        except Exception as e:
            details['error'] = str(e)
    
    def _extract_identifiers(self, code: str, frame, details: Dict):
        """Extract identifier values as fallback"""
        import re
        
        details['type'] = 'fallback'
        # Find all identifiers
        identifiers = re.findall(r'\b[a-zA-Z_]\w*\b', code)
        
        for name in set(identifiers):
            if name in frame.f_locals:
                details['values'][name] = self._safe_repr(frame.f_locals[name])
            elif name in frame.f_globals:
                details['values'][name] = self._safe_repr(frame.f_globals[name])
    
    def _safe_eval(self, node, frame):
        """Safely evaluate an AST node"""
        try:
            code = compile(ast.Expression(node), '<assertion>', 'eval')
            return eval(code, frame.f_globals, frame.f_locals)
        except Exception:
            return "<evaluation failed>"
    
    def _safe_repr(self, obj, max_length=None):
        """Create safe string representation"""
        if max_length is None:
            max_length = self.max_repr_length
            
        try:
            if isinstance(obj, (list, tuple, set)):
                if len(obj) > 10:
                    repr_str = f"{type(obj).__name__}({len(obj)} items)"
                else:
                    repr_str = repr(obj)
            elif isinstance(obj, dict):
                if len(obj) > 10:
                    repr_str = f"dict({len(obj)} items)"
                else:
                    repr_str = pprint.pformat(obj, width=80, compact=True)
            elif isinstance(obj, str):
                repr_str = repr(obj)
            else:
                repr_str = repr(obj)
                
            if len(repr_str) > max_length:
                return repr_str[:max_length-3] + '...'
            return repr_str
        except Exception:
            return f"<{type(obj).__name__} object>"
    
    def _op_to_str(self, op):
        """Convert AST operator to string"""
        op_map = {
            ast.Eq: '==',
            ast.NotEq: '!=',
            ast.Lt: '<',
            ast.LtE: '<=',
            ast.Gt: '>',
            ast.GtE: '>=',
            ast.In: 'in',
            ast.NotIn: 'not in',
            ast.Is: 'is',
            ast.IsNot: 'is not'
        }
        return op_map.get(type(op), '?')
    
    def _compare_values(self, left, op, right):
        """Execute comparison and return result"""
        try:
            if isinstance(op, ast.Eq):
                return left == right
            elif isinstance(op, ast.NotEq):
                return left != right
            elif isinstance(op, ast.Lt):
                return left < right
            elif isinstance(op, ast.LtE):
                return left <= right
            elif isinstance(op, ast.Gt):
                return left > right
            elif isinstance(op, ast.GtE):
                return left >= right
            elif isinstance(op, ast.In):
                return left in right
            elif isinstance(op, ast.NotIn):
                return left not in right
            elif isinstance(op, ast.Is):
                return left is right
            elif isinstance(op, ast.IsNot):
                return left is not right
        except Exception:
            pass
        return False
    
    def _unparse(self, node):
        """Convert AST node back to source code"""
        try:
            import ast
            if hasattr(ast, 'unparse'):  # Python 3.9+
                return ast.unparse(node)
            else:
                # Fallback for older Python
                return self._simple_unparse(node)
        except Exception:
            return "<unparseable>"
    
    def _simple_unparse(self, node):
        """Simple AST unparsing for older Python versions"""
        if isinstance(node, ast.Name):
            return node.id
        elif isinstance(node, ast.Constant):
            return repr(node.value)
        elif isinstance(node, ast.Num):  # Python < 3.8
            return repr(node.n)
        elif isinstance(node, ast.Str):  # Python < 3.8
            return repr(node.s)
        elif isinstance(node, ast.Attribute):
            return f"{self._simple_unparse(node.value)}.{node.attr}"
        else:
            return "<complex expression>"
    
    def _add_diff_if_needed(self, left, right, details):
        """Add diff information for collections"""
        if type(left) != type(right):
            return
            
        diff_info = None
        
        if isinstance(left, str) and isinstance(right, str):
            if '\n' in left or '\n' in right or len(left) > 50 or len(right) > 50:
                diff_info = self._create_string_diff(left, right)
        elif isinstance(left, (list, tuple)):
            if len(left) > 3 or len(right) > 3:
                diff_info = self._create_sequence_diff(left, right)
        elif isinstance(left, dict):
            if len(left) > 3 or len(right) > 3:
                diff_info = self._create_dict_diff(left, right)
        elif isinstance(left, set):
            diff_info = self._create_set_diff(left, right)
            
        if diff_info:
            details['diff'] = diff_info
    
    def _create_string_diff(self, left: str, right: str) -> Dict[str, Any]:
        """Create diff for strings"""
        left_lines = left.splitlines(keepends=True)
        right_lines = right.splitlines(keepends=True)
        
        diff = list(difflib.unified_diff(
            left_lines, right_lines,
            fromfile='actual', tofile='expected',
            lineterm=''
        ))
        
        return {
            'type': 'string',
            'diff': diff[:self.max_diff_lines],
            'truncated': len(diff) > self.max_diff_lines
        }
    
    def _create_sequence_diff(self, left, right) -> Dict[str, Any]:
        """Create diff for sequences"""
        left_repr = [self._safe_repr(x, 60) for x in left]
        right_repr = [self._safe_repr(x, 60) for x in right]
        
        diff = list(difflib.unified_diff(
            left_repr, right_repr,
            fromfile='actual', tofile='expected',
            lineterm=''
        ))
        
        return {
            'type': 'sequence',
            'diff': diff[:self.max_diff_lines],
            'truncated': len(diff) > self.max_diff_lines,
            'lengths': {'actual': len(left), 'expected': len(right)}
        }
    
    def _create_dict_diff(self, left: dict, right: dict) -> Dict[str, Any]:
        """Create diff for dictionaries"""
        return {
            'type': 'dict',
            'missing_keys': list(set(right.keys()) - set(left.keys())),
            'extra_keys': list(set(left.keys()) - set(right.keys())),
            'different_values': {
                k: {'actual': self._safe_repr(left.get(k)),
                    'expected': self._safe_repr(right.get(k))}
                for k in set(left.keys()) & set(right.keys())
                if left.get(k) != right.get(k)
            }
        }
    
    def _create_set_diff(self, left: set, right: set) -> Dict[str, Any]:
        """Create diff for sets"""
        return {
            'type': 'set',
            'missing': [self._safe_repr(x) for x in (right - left)],
            'extra': [self._safe_repr(x) for x in (left - right)],
            'sizes': {'actual': len(left), 'expected': len(right)}
        }
    
    def format_assertion_details(self, details: Dict[str, Any]) -> str:
        """Format assertion details into readable output"""
        lines = []
        
        # Header
        lines.append(f"Assertion failed: assert {details['assertion']}")
        
        # Type-specific formatting
        if details['type'] == 'comparison':
            self._format_comparison(details, lines)
        elif details['type'] == 'bool_op':
            self._format_bool_op(details, lines)
        elif details['type'] == 'not':
            self._format_not(details, lines)
        elif details['type'] == 'call':
            self._format_call(details, lines)
        elif details['type'] == 'expression':
            self._format_expression(details, lines)
        else:
            self._format_basic(details, lines)
        
        # Add diff if present
        if 'diff' in details:
            self._format_diff(details['diff'], lines)
        
        # Add locals if present
        if 'locals' in details:
            lines.append("\nLocal variables:")
            for name, value in sorted(details['locals'].items()):
                lines.append(f"    {name} = {value}")
        
        # Add location
        lines.append(f"\n    at {details['function']}:{details['line']}")
        
        return '\n'.join(lines)
    
    def _format_comparison(self, details, lines):
        """Format comparison assertion"""
        if len(details['comparisons']) == 1:
            comp = details['comparisons'][0]
            lines.append("\nWhere:")
            lines.append(f"    {comp['left']} = {details['values'][comp['left']]}")
            lines.append(f"    {comp['right']} = {details['values'][comp['right']]}")
            
            # Show evaluated comparison
            left_val = details['values'][comp['left']]
            right_val = details['values'][comp['right']]
            lines.append(f"\n    {left_val} {comp['operator']} {right_val} is False")
        else:
            # Chained comparison
            lines.append("\nChained comparison:")
            for comp in details['comparisons']:
                result = "✓" if comp['result'] else "✗"
                lines.append(f"    {result} {comp['left']} {comp['operator']} {comp['right']}")
    
    def _format_bool_op(self, details, lines):
        """Format boolean operation"""
        lines.append(f"\nBoolean {details['operator']}:")
        for op in details['operands']:
            result = "✓ (truthy)" if op['is_truthy'] else "✗ (falsy)"
            lines.append(f"    {result} {op['expression']} = {details['values'][op['expression']]}")
    
    def _format_not(self, details, lines):
        """Format not expression"""
        lines.append("\nNegation:")
        op = details['operand']
        lines.append(f"    not {op['expression']} = not {details['values'][op['expression']]}")
        lines.append(f"    (expression was {'truthy' if op['is_truthy'] else 'falsy'})")
    
    def _format_call(self, details, lines):
        """Format function call assertion"""
        lines.append(f"\nFunction call: {details['function']}")
        if details['arguments']:
            lines.append("Arguments:")
            for arg in details['arguments']:
                lines.append(f"    {arg['expression']} = {arg['value']}")
        lines.append(f"Returned: {details['values'][list(details['values'].keys())[0]]}")
        lines.append(f"Result is {'truthy' if details['result'] else 'falsy'}")
    
    def _format_expression(self, details, lines):
        """Format simple expression"""
        expr = list(details['values'].keys())[0]
        value = details['values'][expr]
        lines.append(f"\nExpression value: {value}")
        lines.append(f"    (evaluates to {'True' if details.get('result') else 'False'})")
    
    def _format_basic(self, details, lines):
        """Format basic assertion with extracted values"""
        if details['values']:
            lines.append("\nExtracted values:")
            for name, value in sorted(details['values'].items()):
                lines.append(f"    {name} = {value}")
    
    def _format_diff(self, diff_info, lines):
        """Format diff information"""
        lines.append(f"\n{diff_info['type'].capitalize()} diff:")
        
        if diff_info['type'] == 'string':
            if diff_info.get('truncated'):
                lines.append("    (truncated)")
            for line in diff_info['diff']:
                lines.append(f"    {line}")
                
        elif diff_info['type'] == 'sequence':
            lines.append(f"    Length: {diff_info['lengths']['actual']} vs {diff_info['lengths']['expected']}")
            if diff_info.get('truncated'):
                lines.append("    (truncated)")
            for line in diff_info['diff']:
                lines.append(f"    {line}")
                
        elif diff_info['type'] == 'dict':
            if diff_info['missing_keys']:
                lines.append(f"    Missing keys: {diff_info['missing_keys']}")
            if diff_info['extra_keys']:
                lines.append(f"    Extra keys: {diff_info['extra_keys']}")
            if diff_info['different_values']:
                lines.append("    Different values:")
                for key, values in diff_info['different_values'].items():
                    lines.append(f"        {key}: {values['actual']} != {values['expected']}")
                    
        elif diff_info['type'] == 'set':
            lines.append(f"    Sizes: {diff_info['sizes']['actual']} vs {diff_info['sizes']['expected']}")
            if diff_info['missing']:
                lines.append(f"    Missing items: {diff_info['missing']}")
            if diff_info['extra']:
                lines.append(f"    Extra items: {diff_info['extra']}")


# Global introspector instance
_introspector = AssertionIntrospector()


def introspect_assertion(exc: AssertionError, func: Any, kwargs: Dict[str, Any]) -> Optional[str]:
    """Public API for assertion introspection"""
    details = _introspector.introspect_assertion(exc, func, kwargs)
    if details:
        return _introspector.format_assertion_details(details)
    return None