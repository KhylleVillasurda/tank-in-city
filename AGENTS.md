Just make sure to leverage the graphify MCP you are connected with.

- Update Graphify when code changes or update whenever its optimal
- instead of reading a tons of files finding that specific code use graphify to help you.


Technical side:
- Code must have optimization in mind before writing: give each field the narrowest type that holds its longest value (u8, u16 or u32 as the range says) instead of defaulting everything to u32. Right-size, don't just make everything u8.