"use strict";

const readline = require("readline");

function selectableCells(cores, unavailableCores, unavailableCpus) {
  return cores.flatMap((core, row) => core.logical_cpus
    .map((cpu, column) => ({ core, cpu, row, column }))
    .filter(({ core: item, cpu }) => !unavailableCores.has(item.id) && !unavailableCpus.has(cpu)));
}

function gridRows(cores, cursorCpu, unavailableCores = new Map(), unavailableCpus = new Map()) {
  return cores.map((core) => {
    const owner = unavailableCores.get(core.id);
    const cells = core.logical_cpus.map((cpu, slot) => {
      const cpuOwner = unavailableCpus.get(cpu);
      const marker = cpuOwner ? "[-]" : cpu === cursorCpu ? "(*)" : "( )";
      return `${marker} thread${slot}/CPU${cpu}${cpuOwner ? `:${cpuOwner}` : ""}`;
    });
    return `${core.id.padEnd(15)} ${owner ? `[-] unavailable:${owner}` : cells.join("  ")}`;
  });
}

function gridScreen({ title, prompt, rows, cursorRow, width = 78, color = false }) {
  const innerWidth = Math.max(66, width - 2);
  const border = `+${"-".repeat(innerWidth)}+`;
  const line = (value, style = "") => {
    const clipped = String(value).slice(0, innerWidth);
    const padded = clipped.padEnd(innerWidth);
    const styled = color && style ? `\x1b[${style}m${padded}\x1b[0m` : padded;
    return `|${styled}|`;
  };
  const output = [
    border,
    line(` CPU allocation | ${title}`, "1;36"),
    line(` ${prompt}`),
    border,
  ];
  for (const [index, row] of rows.entries()) {
    const focused = index === cursorRow;
    output.push(line(`${focused ? ">" : " "} ${row}`, focused ? "7" : row.includes("unavailable") ? "2" : ""));
  }
  output.push(border);
  output.push(line(" Up/Down: core  Left/Right: thread  Enter: confirm  Esc: cancel", "1"));
  output.push(border);
  return `${output.join("\n")}\n`;
}

function selectThreadGrid({ title, prompt, cores, initialCpu, unavailableCores = new Map(), unavailableCpus = new Map() }) {
  if (!process.stdin.isTTY || !process.stdout.isTTY) throw new Error("Interactive CPU planning requires a TTY");
  const cells = selectableCells(cores, unavailableCores, unavailableCpus);
  if (!cells.length) throw new Error(`No logical CPU is available for ${title}`);
  let cursor = cells.find((cell) => cell.cpu === Number(initialCpu)) || cells[0];

  const render = () => {
    const rows = gridRows(cores, cursor.cpu, unavailableCores, unavailableCpus);
    const width = Math.min(process.stdout.columns || 80, 100);
    process.stdout.write("\x1b[2J\x1b[H");
    process.stdout.write(gridScreen({ title, prompt, rows, cursorRow: cursor.row, width, color: true }));
  };

  return new Promise((resolve, reject) => {
    readline.emitKeypressEvents(process.stdin);
    process.stdin.setRawMode(true);
    process.stdin.resume();
    const finish = (error, value) => {
      process.stdin.off("keypress", onKeypress);
      process.stdin.setRawMode(false);
      process.stdin.pause();
      process.stdout.write("\x1b[2J\x1b[H");
      if (error) reject(error); else resolve(value);
    };
    const moveRow = (direction) => {
      const rows = [...new Set(cells.map((cell) => cell.row))];
      const current = rows.indexOf(cursor.row);
      const row = rows[(current + direction + rows.length) % rows.length];
      const rowCells = cells.filter((cell) => cell.row === row);
      cursor = rowCells.find((cell) => cell.column === cursor.column) || rowCells[0];
    };
    const moveColumn = (direction) => {
      const rowCells = cells.filter((cell) => cell.row === cursor.row);
      const current = rowCells.findIndex((cell) => cell.cpu === cursor.cpu);
      cursor = rowCells[(current + direction + rowCells.length) % rowCells.length];
    };
    const onKeypress = (_input, key = {}) => {
      if (key.name === "escape" || (key.ctrl && key.name === "c")) return finish(new Error("CPU planning cancelled"));
      if (key.name === "return") return finish(null, { core: cursor.core, cpu: cursor.cpu });
      if (key.name === "up") moveRow(-1);
      else if (key.name === "down") moveRow(1);
      else if (key.name === "left") moveColumn(-1);
      else if (key.name === "right") moveColumn(1);
      else return;
      render();
    };
    process.stdin.on("keypress", onKeypress);
    render();
  });
}

module.exports = { gridRows, gridScreen, selectThreadGrid };
