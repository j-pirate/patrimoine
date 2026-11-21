// Use the installed compiler, including imports, inheritance and SaveRecord<T>.
import { API, TypeFlags, SymbolFlags } from 'typescript/unstable/sync';
import * as ast from 'typescript/unstable/ast';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[2]);
// Preserve literal values for real Rust wire enums. Other narrowed TS string
// aliases (for example validated display currencies) still model Rust String.
const enumNames = new Set(JSON.parse(process.argv[3] || '[]'));
const directory = path.join(root, 'src/services');
const filenames = fs.readdirSync(directory).filter(name => name === 'api.ts' || name.endsWith('-types.ts'))
  .sort().map(name => path.join(directory, name));
const ts = { ...ast, TypeFlags, SymbolFlags };
const apiClient = new API({ cwd: root });
try {
const snapshot = apiClient.updateSnapshot({ openProjects: [path.join(root, 'tsconfig.app.json')] });
const project = snapshot.getProject(path.join(root, 'tsconfig.app.json'));
if (!project?.compilerOptions.strict && !project?.compilerOptions.strictNullChecks) throw Error('Contract checking requires strictNullChecks');
const { program, checker } = project;
const interfaces = new Map();
const known = new Map();
for (const filename of filenames) {
  const source = program.getSourceFile(filename);
  for (const statement of source.statements) {
    if (!ts.isInterfaceDeclaration(statement) || !statement.modifiers?.some(m => m.kind === ts.SyntaxKind.ExportKeyword)) continue;
    const name = statement.name.text;
    if (interfaces.has(name)) throw Error(`Duplicate contract interface ${name}`);
    interfaces.set(name, statement);
    known.set(checker.getSymbolAtLocation(statement.name), name);
  }
}
function shape(type, expand = false) {
  const flags = type.flags;
  if (enumNames.has(type.getAliasSymbol()?.name)) {
    const values = type.isUnionType() ? type.getTypes() : [type];
    if (values.some(t => !(t.flags & ts.TypeFlags.StringLiteral))) throw Error('Rust enum aliases must contain only string literals');
    return { enum: values.map(t => t.value).sort() };
  }
  if (flags & ts.TypeFlags.StringLike) return 'string';
  if (flags & ts.TypeFlags.NumberLike) return 'number';
  if (flags & ts.TypeFlags.BooleanLike) return 'boolean';
  if (flags & ts.TypeFlags.Null) return 'null';
  if (flags & ts.TypeFlags.Undefined) return 'undefined';
  if (flags & ts.TypeFlags.Void) return 'void';
  if (type.isUnionType()) {
    const choices = type.getTypes().map(t => shape(t));
    const unique = [...new Map(choices.map(c => [JSON.stringify(c), c])).values()];
    return unique.length === 1 ? unique[0] : { union: unique.sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b))) };
  }
  if (checker.isArrayType(type)) return { array: shape(checker.getTypeArguments(type)[0]) };
  if (!expand && known.has(type.getSymbol())) return { ref: known.get(type.getSymbol()) };
  if (flags & ts.TypeFlags.Object || type.isIntersectionType()) {
    const indices = checker.getIndexInfosOfType(type);
    if (indices.length) {
      if (indices.length !== 1 || !(indices[0].keyType.flags & ts.TypeFlags.StringLike)) throw Error('Only string-keyed contract maps are supported');
      return { map: shape(indices[0].valueType) };
    }
    const fields = {};
    for (const property of checker.getPropertiesOfType(type)) {
      const required = !(property.flags & ts.SymbolFlags.Optional);
      let value = shape(checker.getTypeOfSymbol(property));
      if (!required && value.union?.includes('undefined')) {
        const values = value.union.filter(v => v !== 'undefined');
        value = values.length === 1 ? values[0] : { union: values };
      }
      fields[property.name] = { required, type: value };
    }
    return { fields };
  }
  throw Error(`Unsupported TypeScript contract type ${checker.typeToString(type)}`);
}
const types = Object.fromEntries([...interfaces].map(([name, node]) => [name, shape(checker.getTypeAtLocation(node), true)]));
const commands = [];
const api = program.getSourceFile(path.join(directory, 'api.ts'));
function visit(node) {
  if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === 'invoke') {
    if (node.typeArguments?.length !== 1 || !ts.isStringLiteral(node.arguments[0]) || node.arguments.length > 2) throw Error('Unsupported invoke expression');
    const argumentsNode = node.arguments[1];
    if (argumentsNode && !ts.isObjectLiteralExpression(argumentsNode)) throw Error('Invoke arguments must be an object literal');
    if (argumentsNode?.properties.some(p => ts.isSpreadAssignment(p))) throw Error('Invoke spreads require explicit schema support');
    commands.push({ name: node.arguments[0].text, args: argumentsNode ? shape(checker.getTypeAtLocation(argumentsNode), true) : { fields: {} }, return: shape(checker.getTypeFromTypeNode(node.typeArguments[0])) });
  }
  node.forEachChild(visit);
}
visit(api);
const errorAlias = api.statements.find(s => ts.isTypeAliasDeclaration(s) && s.name.text === 'ErrorCode');
if (!errorAlias) throw Error('Missing ErrorCode alias');
const errorType = checker.getTypeAtLocation(errorAlias);
if (!errorType.isUnionType() || errorType.getTypes().some(t => !(t.flags & ts.TypeFlags.StringLiteral))) throw Error('ErrorCode must be a literal union');
console.log(JSON.stringify({ types, commands, error_codes: errorType.getTypes().map(t => t.value).sort() }));
} finally {
  apiClient.close();
}
