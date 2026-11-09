import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

const [debugPort, serverUrl, roomCode] = process.argv.slice(2);
if (!debugPort || !serverUrl || !roomCode) {
  throw new Error("Usage: node Verify-RoomPanel.mjs <debugPort> <serverUrl> <roomCode>");
}

const sleep = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
const targetsUrl = `http://127.0.0.1:${debugPort}/json/list`;
let target;
for (let attempt = 0; attempt < 60; attempt++) {
  try {
    const targets = await (await fetch(targetsUrl)).json();
    target = targets.find(item => item.type === "page");
    if (target?.webSocketDebuggerUrl) break;
  } catch { /* Edge is still starting. */ }
  await sleep(250);
}
if (!target?.webSocketDebuggerUrl) throw new Error("Edge DevTools page was not found.");

const socket = new WebSocket(target.webSocketDebuggerUrl);
const pending = new Map();
const pageErrors = [];
let nextId = 0;
socket.addEventListener("message", event => {
  const message = JSON.parse(event.data);
  if (message.method === "Runtime.exceptionThrown") {
    pageErrors.push(message.params?.exceptionDetails?.exception?.description ||
      message.params?.exceptionDetails?.text || "Uncaught browser exception");
  }
  if (message.method === "Runtime.consoleAPICalled" && message.params?.type === "error") {
    pageErrors.push(message.params.args?.map(item => item.value || item.description || "").join(" "));
  }
  if (message.id && pending.has(message.id)) {
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  }
});
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});

function command(method, params = {}) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      if (pending.delete(id)) reject(new Error(`Timed out waiting for ${method}.`));
    }, 15000);
    pending.set(id, {
      resolve: value => { clearTimeout(timer); resolve(value); },
      reject: error => { clearTimeout(timer); reject(error); }
    });
    socket.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const response = await command("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true
  });
  if (response.exceptionDetails) {
    throw new Error(response.exceptionDetails.exception?.description ||
      response.exceptionDetails.text || "Page JavaScript evaluation failed.");
  }
  return response.result.value;
}

async function waitFor(expression, label) {
  for (let attempt = 0; attempt < 60; attempt++) {
    if (await evaluate(expression)) return;
    await sleep(200);
  }
  throw new Error(`Timed out waiting for ${label}.`);
}

async function roomSnapshot() {
  const response = await fetch(`${serverUrl}/api/room?roomCode=${encodeURIComponent(roomCode)}`);
  if (!response.ok) throw new Error(`Room API returned HTTP ${response.status}.`);
  return response.json();
}

await command("Page.enable");
await command("Runtime.enable");
await command("Log.enable");
await command("Emulation.setDeviceMetricsOverride", {
  width: 390,
  height: 844,
  deviceScaleFactor: 1,
  mobile: true
});
await command("Page.navigate", { url: serverUrl });
await waitFor('document.querySelector("#joinForm") !== null', "web panel document");
await waitFor('document.fonts.check("14px Inter")', "locally bundled Inter font");
const manualJoin = await evaluate(`({
  hasServerField: document.querySelector("#serverUrl") !== null,
  codeVisible: !document.querySelector("#roomCodeField").hidden,
  codeRequired: document.querySelector("#roomCode").required
})`);
if (manualJoin.hasServerField || !manualJoin.codeVisible || !manualJoin.codeRequired) {
  throw new Error("A room opened without an invite should ask only for its code and the participant name.");
}
const joinScreenshotDirectory = path.resolve(".taste-skill", "live-code-room", "join");
await mkdir(joinScreenshotDirectory, { recursive: true });
const mobileJoinCapture = await command("Page.captureScreenshot", {
  format: "png", fromSurface: true, captureBeyondViewport: false
});
await writeFile(path.join(joinScreenshotDirectory, "mobile-390.png"), Buffer.from(mobileJoinCapture.data, "base64"));
await command("Emulation.setDeviceMetricsOverride", {
  width: 1440, height: 900, deviceScaleFactor: 1, mobile: false
});
const desktopJoinCapture = await command("Page.captureScreenshot", {
  format: "png", fromSurface: true, captureBeyondViewport: false
});
await writeFile(path.join(joinScreenshotDirectory, "desktop-1440.png"), Buffer.from(desktopJoinCapture.data, "base64"));
await command("Emulation.setDeviceMetricsOverride", {
  width: 390, height: 844, deviceScaleFactor: 1, mobile: true
});
const invalidInvite = new URL(serverUrl);
invalidInvite.searchParams.set("roomCode", "000000");
await command("Page.navigate", { url: invalidInvite.toString() });
await waitFor('document.querySelector("#joinForm") !== null', "invalid invite link form");
await evaluate(`(() => {
  document.querySelector("#actorName").value = "Проверка E2E";
  document.querySelector("#joinForm").requestSubmit();
})()`);
await waitFor('document.querySelector("#joinError").textContent.includes("Попросите ведущего")', "helpful invalid room code message");
await command("Page.navigate", { url: serverUrl });
await waitFor('document.querySelector("#joinForm") !== null', "manual room code form");
await evaluate(`(() => {
  document.querySelector("#roomCode").value = ${JSON.stringify(roomCode)};
  document.querySelector("#actorName").value = "Проверка E2E";
  document.querySelector("#joinForm").requestSubmit();
})()`);
await waitFor('!document.querySelector("#dashboard").classList.contains("hidden")', "manual code room connection");
await evaluate('document.querySelector("#disconnectButton").click()');
await waitFor('!document.querySelector("#joinPanel").classList.contains("hidden")', "manual room disconnect");
const inviteUrl = new URL(serverUrl);
inviteUrl.searchParams.set("roomCode", roomCode);
await command("Page.navigate", { url: inviteUrl.toString() });
await waitFor('document.querySelector("#joinForm") !== null', "invite link join form");
await waitFor('document.querySelector("#roomCode").value === ' + JSON.stringify(roomCode), "room code from invite link");
const inviteJoin = await evaluate(`({
  codeHidden: document.querySelector("#roomCodeField").hidden,
  nameVisible: document.querySelector("#actorName").getClientRects().length > 0,
  origin: window.location.origin
})`);
if (!inviteJoin.codeHidden || !inviteJoin.nameVisible || inviteJoin.origin !== new URL(serverUrl).origin) {
  throw new Error("An invite link should fill and hide the room code and keep the panel on the same server.");
}
await evaluate(`(() => {
  document.querySelector("#actorName").value = "Проверка E2E";
  document.querySelector("#joinForm").requestSubmit();
})()`);
await waitFor('!document.querySelector("#dashboard").classList.contains("hidden")', "web panel connection");
const joinedLink = await evaluate('new URL(window.location.href).searchParams.get("roomCode")');
if (joinedLink !== roomCode) throw new Error("Joining the room should keep its shareable code in the address.");
await evaluate(`(() => {
  const originalFetch = window.fetch.bind(window);
  window.fetch = (input, options) => {
    const requestUrl = new URL(input, window.location.href);
    if (requestUrl.pathname === "/api/info") {
      return Promise.resolve(new Response(JSON.stringify({ roomInviteBases: ["http://192.168.1.20:5080"] }), {
        status: 200,
        headers: { "Content-Type": "application/json" }
      }));
    }
    return originalFetch(input, options);
  };
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: async text => { window.__copiedInviteLink = text; } }
  });
  document.querySelector("#copyInviteButton").click();
})()`);
await waitFor('document.querySelector("#inviteStatus").textContent.includes("Ссылка скопирована")', "invite link copy confirmation");
const copiedInvite = await evaluate('window.__copiedInviteLink');
const expectedShareUrl = new URL("http://192.168.1.20:5080");
expectedShareUrl.searchParams.set("roomCode", roomCode);
if (copiedInvite !== expectedShareUrl.toString()) {
  throw new Error("Copying from localhost should use the server's network link and room code.");
}
await waitFor('[...document.querySelectorAll(".participant-name")].some(item => item.textContent.trim() === "Саят")', "existing participant");

const mobileLayout = await evaluate(`({
  width: document.documentElement.clientWidth,
  scrollWidth: document.documentElement.scrollWidth,
  count: document.querySelectorAll(".participant-card").length,
  listVisible: document.querySelector(".participants-panel").getClientRects().length > 0,
  detailVisible: document.querySelector(".main-column").getClientRects().length > 0
})`);
if (mobileLayout.scrollWidth > mobileLayout.width + 1) {
  throw new Error(`Mobile layout overflows horizontally (${mobileLayout.scrollWidth}px > ${mobileLayout.width}px).`);
}
if (mobileLayout.count < 2) throw new Error("The web panel did not display the joining participant alongside Саят.");
if (!mobileLayout.listVisible || mobileLayout.detailVisible) {
  throw new Error("Mobile entry should show the participant list before its detail.");
}

await evaluate('(() => { const card = [...document.querySelectorAll(".participant-card")].find(item => item.querySelector(".participant-name")?.textContent.trim() === "Саят"); if (!card) throw new Error("Participant Саят is missing from the mobile list."); card.click(); })()');
await waitFor(
  'document.querySelector("#workbenchGrid").classList.contains("mobile-detail-open") && getComputedStyle(document.querySelector(".main-column")).display !== "none"',
  "mobile participant detail"
);
const mobileDetail = await evaluate(`({
  width: document.documentElement.clientWidth,
  scrollWidth: document.documentElement.scrollWidth,
  listVisible: document.querySelector(".participants-panel").getClientRects().length > 0,
  detailVisible: document.querySelector(".main-column").getClientRects().length > 0,
  focusedComposer: document.activeElement === document.querySelector("#quickText")
})`);
if (mobileDetail.scrollWidth > mobileDetail.width + 1) {
  throw new Error("Selected-participant view overflows horizontally (" + mobileDetail.scrollWidth + "px > " + mobileDetail.width + "px).");
}
if (mobileDetail.listVisible || !mobileDetail.detailVisible || !mobileDetail.focusedComposer) {
  throw new Error("Selecting a participant should open the composer and focus its text field.");
}
await evaluate('document.querySelector("#showLibraryButton").click()');
await waitFor(
  'document.querySelector("#workbenchGrid").classList.contains("mobile-library-open") && document.querySelector(".library-panel").getClientRects().length > 0 && document.querySelector(".quick-send-panel").getClientRects().length === 0',
  "mobile task library"
);
await evaluate('document.querySelector("#backToSendButton").click()');
await waitFor(
  '!document.querySelector("#workbenchGrid").classList.contains("mobile-library-open") && document.querySelector(".quick-send-panel").getClientRects().length > 0',
  "mobile return to text composer"
);
await evaluate('document.querySelector("#backToParticipants").click()');
await waitFor(
  '!document.querySelector("#workbenchGrid").classList.contains("mobile-detail-open") && document.querySelector(".main-column").getClientRects().length === 0',
  "mobile participant list return"
);
console.log("PASS: mobile layout fits 390px and switches between the real participant list and selected-recipient composer.");

await command("Emulation.setDeviceMetricsOverride", {
  width: 1440,
  height: 1000,
  deviceScaleFactor: 1,
  mobile: false
});
const desktopLayout = await evaluate(`({
  width: document.documentElement.clientWidth,
  scrollWidth: document.documentElement.scrollWidth,
  columns: getComputedStyle(document.querySelector(".dashboard-grid")).gridTemplateColumns
})`);
if (desktopLayout.scrollWidth > desktopLayout.width + 1) {
  throw new Error(`Desktop layout overflows horizontally (${desktopLayout.scrollWidth}px > ${desktopLayout.width}px).`);
}
if (desktopLayout.columns.trim().split(" ").filter(Boolean).length < 2) {
  throw new Error("The wide-screen dashboard did not use multiple columns.");
}
await command("Emulation.setDeviceMetricsOverride", {
  width: 768,
  height: 1024,
  deviceScaleFactor: 1,
  mobile: true
});
const tabletLayout = await evaluate(`({
  width: document.documentElement.clientWidth,
  scrollWidth: document.documentElement.scrollWidth,
  columns: getComputedStyle(document.querySelector(".dashboard-grid")).gridTemplateColumns
})`);
if (tabletLayout.scrollWidth > tabletLayout.width + 1) {
  throw new Error("Tablet layout overflows horizontally (" + tabletLayout.scrollWidth + "px > " + tabletLayout.width + "px).");
}
if (tabletLayout.columns.trim().split(" ").filter(Boolean).length < 2) {
  throw new Error("The tablet workbench did not keep participants beside the composer.");
}
await command("Emulation.setDeviceMetricsOverride", {
  width: 1440,
  height: 900,
  deviceScaleFactor: 1,
  mobile: false
});

await evaluate('window.confirm = () => true');
await evaluate(`(() => {
  const card = [...document.querySelectorAll(".participant-card")]
    .find(item => item.querySelector(".participant-name")?.textContent.trim() === "Саят");
  if (!card) throw new Error("Participant Саят is missing.");
  card.click();
  document.querySelector("#newTaskButton").click();
  document.querySelector("#taskNumber").value = "12";
  document.querySelector("#taskTitle").value = "Сумма двух чисел";
  document.querySelector("#taskLanguage").value = "C#";
  document.querySelector("#taskCode").value = "using System;\\nConsole.WriteLine(12);";
  document.querySelector("#taskForm").requestSubmit();
})()`);
await waitFor('document.querySelector("#taskDialog").open === false && document.querySelectorAll(".task-card").length === 1', "task creation from the web form");
await evaluate('document.querySelector(".assign-button").click()');
await waitFor('document.querySelector(".participant-card.selected .participant-name")?.textContent.trim() === "Саят"', "task assignment UI refresh");

let snapshot;
let participant;
let task;
for (let attempt = 0; attempt < 40; attempt++) {
  snapshot = await roomSnapshot();
  participant = snapshot.participants.find(item => item.name === "Саят");
  task = snapshot.tasks.find(item => item.number === "12");
  if (participant?.assignedTaskId === task?.id) break;
  await sleep(150);
}
if (!participant || participant.assignedTaskId !== task?.id) {
  throw new Error("Assigning the task in the web panel did not update Саят's assignment.");
}

const quickText = "Сәлем, Саят!\nConsole.WriteLine(12); 🌟";
await evaluate(`(() => {
  const text = document.querySelector("#quickText");
  text.value = ${JSON.stringify(quickText)};
  text.dispatchEvent(new Event("input", { bubbles: true }));
  document.querySelector("#quickSendButton").click();
})()`);
await waitFor('document.querySelector("#quickSendStatus").textContent.includes("Текст отправлен")', "quick text send from the web panel");
snapshot = await roomSnapshot();
participant = snapshot.participants.find(item => item.name === "Саят");
const deliveryResponse = await fetch(`${serverUrl}/api/poll`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ roomCode, participantId: participant?.id })
});
const delivery = await deliveryResponse.json();
if (delivery.assignedTask?.code !== quickText || !delivery.assignedTask.isQuickSend) {
  throw new Error("Quick send mismatch: " + JSON.stringify({
    expected: quickText,
    participant,
    delivery,
    snapshotTaskIds: snapshot.tasks.map(item => item.id)
  }));
}

console.log("PASS: browser form creates a task, assigns it to the selected participant, and sends arbitrary multiline Unicode text.");
console.log("PASS: wide-screen dashboard uses multiple columns without horizontal overflow.");

const screenshotDirectory = path.resolve(".taste-skill", "live-code-room", "dashboard");
await mkdir(screenshotDirectory, { recursive: true });
async function captureScreenshot(name) {
  await evaluate("window.scrollTo(0, 0)");
  const capture = await command("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
    captureBeyondViewport: false
  });
  await writeFile(path.join(screenshotDirectory, name), Buffer.from(capture.data, "base64"));
}

await captureScreenshot("desktop-1440.png");
await command("Emulation.setDeviceMetricsOverride", {
  width: 768,
  height: 1024,
  deviceScaleFactor: 1,
  mobile: true
});
const finalTablet = await evaluate(`({
  width: document.documentElement.clientWidth,
  scrollWidth: document.documentElement.scrollWidth
})`);
if (finalTablet.scrollWidth > finalTablet.width + 1) {
  throw new Error("Final tablet screen overflows horizontally (" + finalTablet.scrollWidth + "px > " + finalTablet.width + "px).");
}
await captureScreenshot("tablet-768.png");
await command("Emulation.setDeviceMetricsOverride", {
  width: 390,
  height: 844,
  deviceScaleFactor: 1,
  mobile: true
});
await evaluate('(() => { const card = [...document.querySelectorAll(".participant-card")].find(item => item.querySelector(".participant-name")?.textContent.trim() === "Саят"); if (!card) throw new Error("Participant Саят is missing from the final screenshot."); card.click(); })()');
await waitFor(
  'document.querySelector("#workbenchGrid").classList.contains("mobile-detail-open")',
  "final mobile selected-recipient screen"
);
await captureScreenshot("mobile-390-detail.png");
await evaluate('document.querySelector("#showLibraryButton").click()');
await waitFor(
  'document.querySelector("#workbenchGrid").classList.contains("mobile-library-open")',
  "final mobile task library"
);
await captureScreenshot("mobile-390-library.png");
await evaluate('document.querySelector("#backToSendButton").click()');
await waitFor(
  '!document.querySelector("#workbenchGrid").classList.contains("mobile-library-open")',
  "final mobile return to sending"
);
await evaluate('document.querySelector("#backToParticipants").click()');
await waitFor(
  '!document.querySelector("#workbenchGrid").classList.contains("mobile-detail-open")',
  "final mobile participant-list screen"
);
await captureScreenshot("mobile-390-list.png");
console.log("PASS: captured the real room panel at 1440px, 768px, and 390px after the browser-driven workflow.");
if (pageErrors.length) {
  throw new Error("Browser reported JavaScript errors: " + pageErrors.join(" | "));
}
console.log("PASS: browser reported no JavaScript exceptions or console errors.");
socket.close();
