(() => {
  const joinPanel = document.querySelector("#joinPanel");
  const dashboard = document.querySelector("#dashboard");
  const joinForm = document.querySelector("#joinForm");
  const joinError = document.querySelector("#joinError");
  const roomCodeInput = document.querySelector("#roomCode");
  const roomCodeField = document.querySelector("#roomCodeField");
  const inviteStatus = document.querySelector("#inviteStatus");
  const copyInviteButton = document.querySelector("#copyInviteButton");
  const taskDialog = document.querySelector("#taskDialog");
  const taskForm = document.querySelector("#taskForm");
  const taskError = document.querySelector("#taskError");
  const connectionBadge = document.querySelector("#connectionBadge");
  const quickTextBox = document.querySelector("#quickText");
  const quickSendStatus = document.querySelector("#quickSendStatus");
  const quickSendButton = document.querySelector("#quickSendButton");
  const workbenchGrid = document.querySelector("#workbenchGrid");
  const backToParticipants = document.querySelector("#backToParticipants");
  const showLibraryButton = document.querySelector("#showLibraryButton");
  const backToSendButton = document.querySelector("#backToSendButton");

  let apiBase = "";
  let roomCode = "";
  let actorName = "";
  let participantId = "";
  let snapshot = { tasks: [], participants: [], actions: [] };
  let selectedParticipantId = "";
  let pollTimer = null;
  let quickSendInProgress = false;

  const invitedRoomCode = new URLSearchParams(window.location.search).get("roomCode")?.trim() || "";
  if (invitedRoomCode) {
    roomCodeInput.value = invitedRoomCode;
    roomCodeField.hidden = true;
  } else {
    roomCodeInput.required = true;
  }
  copyInviteButton.addEventListener("click", copyInviteLink);
  document.querySelector("#disconnectButton").addEventListener("click", disconnect);
  document.querySelector("#newTaskButton").addEventListener("click", () => openTaskEditor());
  document.querySelector("#closeTaskDialog").addEventListener("click", closeTaskDialog);
  document.querySelector("#cancelTaskButton").addEventListener("click", closeTaskDialog);
  document.querySelector("#pauseButton").addEventListener("click", () => sendCommand("pause"));
  document.querySelector("#restartButton").addEventListener("click", () => sendCommand("restart"));
  quickSendButton.addEventListener("click", sendQuickText);
  quickTextBox.addEventListener("input", updateQuickTextCount);
  backToParticipants.addEventListener("click", showParticipantList);
  showLibraryButton.addEventListener("click", showTaskLibrary);
  backToSendButton.addEventListener("click", showTextComposer);
  joinForm.addEventListener("submit", connect);
  taskForm.addEventListener("submit", saveTask);

  async function connect(event) {
    event.preventDefault();
    joinError.textContent = "";
    apiBase = window.location.origin;
    roomCode = roomCodeInput.value.trim();
    actorName = document.querySelector("#actorName").value.trim();
    if (!roomCode || !actorName) return;

    try {
      const participantKey = getParticipantStorageKey();
      let savedParticipantId = null;
      try { savedParticipantId = window.sessionStorage.getItem(participantKey); } catch { /* Storage can be disabled in private browser modes. */ }
      const joined = await request("/api/join", {
        method: "POST",
        body: JSON.stringify({
          roomCode,
          name: actorName,
          participantId: savedParticipantId || null
        })
      });
      participantId = joined.participantId;
      try { window.sessionStorage.setItem(participantKey, participantId); } catch { /* Joining does not depend on browser storage. */ }
      const inviteUrl = new URL(window.location.href);
      inviteUrl.search = "";
      inviteUrl.searchParams.set("roomCode", roomCode);
      inviteUrl.hash = "";
      window.history.replaceState(null, "", inviteUrl);
      document.querySelector("#roomCodeLabel").textContent = roomCode;
      document.querySelector("#actorLabel").textContent = actorName;
      joinPanel.classList.add("hidden");
      dashboard.classList.remove("hidden");
      connectionBadge.classList.add("online");
      connectionBadge.lastElementChild.textContent = "Подключено";
      await refresh();
      pollTimer = window.setInterval(refresh, 1300);
    } catch (error) {
      joinError.textContent = error instanceof TypeError
        ? "Не удалось связаться с комнатой. Проверьте, что ведущий запустил сервер и вы подключены к той же сети Wi‑Fi."
        : error.message || "Не удалось подключиться к комнате.";
    }
  }

  async function copyInviteLink() {
    copyInviteButton.disabled = true;
    try {
      const currentUrl = new URL(window.location.href);
      let inviteOrigin = currentUrl.origin;
      const localHost = currentUrl.hostname.toLowerCase();
      const isLoopback = localHost === "localhost" || localHost === "::1" ||
        localHost === "[::1]" || localHost === "0.0.0.0" || localHost.startsWith("127.");
      if (isLoopback) {
        try {
          const info = await request("/api/info");
          inviteOrigin = info.roomInviteBases?.find(base => {
            try {
              const host = new URL(base).hostname.toLowerCase();
              return host !== "localhost" && host !== "::1" && host !== "[::1]" &&
                host !== "0.0.0.0" && !host.startsWith("127.");
            } catch { return false; }
          }) || "";
        } catch { inviteOrigin = ""; }
      }
      if (!inviteOrigin) {
        inviteStatus.textContent = "Не удалось найти адрес сети. Откройте веб-панель по ссылке для участников в окне сервера.";
        return;
      }
      const inviteUrl = new URL(inviteOrigin);
      inviteUrl.searchParams.set("roomCode", roomCode);
      const text = inviteUrl.toString();
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const field = document.createElement("textarea");
        field.value = text;
        field.setAttribute("readonly", "");
        field.style.position = "fixed";
        field.style.opacity = "0";
        document.body.append(field);
        field.select();
        const copied = document.execCommand("copy");
        field.remove();
        if (!copied) throw new Error("clipboard unavailable");
      }
      inviteStatus.textContent = "Ссылка скопирована. Отправьте её группе.";
    } catch {
      inviteStatus.textContent = "Скопируйте ссылку из адресной строки браузера и отправьте её группе.";
    } finally {
      copyInviteButton.disabled = false;
      window.setTimeout(() => { inviteStatus.textContent = ""; }, 5000);
    }
  }

  function getParticipantStorageKey() {
    return "liveCodeTyper:participant:" + apiBase + ":" + roomCode + ":" + actorName;
  }

  async function disconnect() {
    window.clearInterval(pollTimer);
    pollTimer = null;
    const leavingParticipantId = participantId;
    participantId = "";
    if (leavingParticipantId) {
      try {
        await request("/api/leave", {
          method: "POST",
          body: JSON.stringify({ roomCode, participantId: leavingParticipantId })
        });
      } catch { /* The panel still disconnects locally when the server is offline. */ }
    }
    dashboard.classList.add("hidden");
    workbenchGrid.classList.remove("mobile-detail-open");
    joinPanel.classList.remove("hidden");
    connectionBadge.classList.remove("online");
    connectionBadge.lastElementChild.textContent = "Не подключено";
  }

  async function refresh() {
    try {
      if (participantId) {
        await request("/api/poll", {
          method: "POST",
          body: JSON.stringify({ roomCode, participantId })
        });
      }
      snapshot = await request("/api/room?roomCode=" + encodeURIComponent(roomCode));
      connectionBadge.classList.add("online");
      connectionBadge.lastElementChild.textContent = "Подключено";
      render();
    } catch {
      connectionBadge.classList.remove("online");
      connectionBadge.lastElementChild.textContent = "Нет связи";
      quickSendButton.disabled = true;
    }
  }

  async function request(path, options = {}) {
    const response = await fetch(apiBase + path, {
      ...options,
      headers: { "Content-Type": "application/json", ...(options.headers || {}) }
    });
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      if (response.status === 401) throw new Error("Код комнаты не подходит. Попросите ведущего отправить актуальную ссылку.");
      throw new Error(body.error || "Ошибка сервера (" + response.status + ").");
    }
    if (response.status === 204) return null;
    const text = await response.text();
    return text ? JSON.parse(text) : null;
  }

  function render() {
    renderParticipants();
    renderTasks();
    renderActions();
  }

  function renderParticipants() {
    const list = document.querySelector("#participantsList");
    const focusedParticipantId = list.contains(document.activeElement)
      ? document.activeElement.dataset.participantId
      : "";
    list.replaceChildren();
    const participants = snapshot.participants || [];
    document.querySelector("#participantCount").textContent = participants.length;
    document.querySelector("#emptyParticipants").classList.toggle("hidden", participants.length > 0);

    if (!participants.some(item => item.id === selectedParticipantId))
      selectedParticipantId = participants[0]?.id || "";

    for (const participant of participants) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "participant-card" + (participant.id === selectedParticipantId ? " selected" : "");
      button.dataset.participantId = participant.id;
      button.setAttribute("aria-pressed", String(participant.id === selectedParticipantId));
      button.setAttribute("aria-label",
        participant.name + ", " +
        (participant.isOnline ? "в сети" : "не в сети") + ", " +
        (participant.assignedTaskLabel || "задание не назначено"));
      button.addEventListener("click", () => {
        selectedParticipantId = participant.id;
        setQuickSendStatus("");
        workbenchGrid.classList.remove("mobile-library-open");
        for (const card of list.querySelectorAll(".participant-card")) {
          const selected = card.dataset.participantId === selectedParticipantId;
          card.classList.toggle("selected", selected);
          card.setAttribute("aria-pressed", String(selected));
        }
        workbenchGrid.classList.add("mobile-detail-open");
        renderSelectedParticipant();
        renderTasks();
        quickTextBox.focus({ preventScroll: true });
      });

      const avatar = document.createElement("span");
      avatar.className = "avatar";
      avatar.textContent = participant.name.trim().slice(0, 1).toUpperCase() || "?";
      const copy = document.createElement("span");
      copy.className = "participant-copy";
      const name = document.createElement("span");
      name.className = "participant-name";
      name.textContent = participant.name;
      const task = document.createElement("span");
      task.className = "participant-task";
      task.textContent = participant.assignedTaskLabel || "Задание не назначено";
      copy.append(name, task);
      const state = document.createElement("span");
      state.className = "online-label" + (participant.isOnline ? " is-online" : "");
      const dot = document.createElement("span");
      dot.className = "dot";
      state.append(dot, document.createTextNode(participant.isOnline ? "В сети" : "Не в сети"));
      button.append(avatar, copy, state);
      list.append(button);
    }

    if (focusedParticipantId) {
      [...list.querySelectorAll(".participant-card")]
        .find(card => card.dataset.participantId === focusedParticipantId)
        ?.focus();
    }
    if (!selectedParticipantId) workbenchGrid.classList.remove("mobile-detail-open");
    renderSelectedParticipant();
    const selected = participants.find(item => item.id === selectedParticipantId);
    const hasSelection = Boolean(selected);
    document.querySelector("#pauseButton").disabled = !hasSelection;
    document.querySelector("#restartButton").disabled = !hasSelection;
    quickSendButton.disabled = !selected?.isOnline || quickSendInProgress;
  }

  function renderSelectedParticipant() {
    const participant = snapshot.participants.find(item => item.id === selectedParticipantId);
    const name = document.querySelector("#selectedParticipantName");
    const initial = document.querySelector("#selectedParticipantInitial");
    const assignment = document.querySelector("#selectedParticipantAssignment");
    name.textContent = participant?.name || "Выберите участника";
    initial.textContent = participant?.name.trim().slice(0, 1).toUpperCase() || "?";
    assignment.textContent = participant
      ? "Текущее назначение: " + (participant.assignedTaskLabel || "нет")
      : "Сначала выберите участника";
  }

  function showParticipantList() {
    workbenchGrid.classList.remove("mobile-detail-open");
    workbenchGrid.classList.remove("mobile-library-open");
    document.querySelector("#participantsList .participant-card")?.focus({ preventScroll: true });
  }

  function showTaskLibrary() {
    workbenchGrid.classList.add("mobile-library-open");
    document.querySelector("#newTaskButton").focus({ preventScroll: true });
  }

  function showTextComposer() {
    workbenchGrid.classList.remove("mobile-library-open");
    quickTextBox.focus({ preventScroll: true });
  }

  function updateQuickTextCount() {
    document.querySelector("#quickTextCount").textContent =
      quickTextBox.value.length.toLocaleString("ru-RU") + " / 1 000 000";
  }

  function setQuickSendStatus(message, isError = false) {
    quickSendStatus.textContent = message;
    quickSendStatus.classList.toggle("is-error", isError);
  }

  async function sendQuickText() {
    setQuickSendStatus("");
    const participant = snapshot.participants.find(item => item.id === selectedParticipantId);
    if (!participant) {
      setQuickSendStatus("Сначала выбери участника слева.", true);
      return;
    }

    if (!participant.isOnline) {
      setQuickSendStatus("Этот участник сейчас не в сети.", true);
      return;
    }

    const text = quickTextBox.value;
    if (text.length === 0) {
      setQuickSendStatus("Вставь текст перед отправкой.", true);
      return;
    }

    if (participant.assignedTaskId &&
        !window.confirm("Отправить новый текст для " + participant.name + "? Текущее назначение заменится."))
      return;

    quickSendInProgress = true;
    quickSendButton.disabled = true;
    try {
      await request("/api/quick-send", {
        method: "POST",
        body: JSON.stringify({ roomCode, actorName, participantId: participant.id, text })
      });
      quickTextBox.value = "";
      updateQuickTextCount();
      await refresh();
      setQuickSendStatus("Текст отправлен " + participant.name + " (" + text.length.toLocaleString("ru-RU") + " символов).");
    } catch (error) {
      setQuickSendStatus(error.message, true);
    } finally {
      quickSendInProgress = false;
      const participant = snapshot.participants.find(item => item.id === selectedParticipantId);
      quickSendButton.disabled = !participant?.isOnline;
    }
  }

  function renderTasks() {
    const list = document.querySelector("#tasksList");
    list.replaceChildren();
    const tasks = snapshot.tasks || [];
    document.querySelector("#emptyTasks").classList.toggle("hidden", tasks.length > 0);

    for (const task of tasks) {
      const card = document.createElement("article");
      card.className = "task-card";
      const copy = document.createElement("div");
      copy.className = "task-copy";
      const title = document.createElement("div");
      title.className = "task-title";
      title.textContent = "№" + task.number + " · " + task.title;
      const meta = document.createElement("div");
      meta.className = "task-meta";
      meta.textContent = task.language + " · " + task.code.length.toLocaleString("ru-RU") + " символов";
      copy.append(title, meta);

      const actions = document.createElement("div");
      actions.className = "task-actions";
      const assign = makeButton(
        selectedParticipantId ? "Назначить" : "Выберите человека",
        "secondary-button assign-button",
        () => assignTask(task));
      assign.disabled = !selectedParticipantId;
      const edit = makeButton("Изменить", "secondary-button edit-button", () => openTaskEditor(task));
      const remove = makeButton("Удалить", "secondary-button delete-button", () => deleteTask(task));
      actions.append(assign, edit, remove);
      card.append(copy, actions);
      list.append(card);
    }
  }

  function renderActions() {
    const list = document.querySelector("#actionsList");
    list.replaceChildren();
    const actions = snapshot.actions || [];
    if (actions.length === 0) {
      const empty = document.createElement("div");
      empty.className = "empty-state";
      empty.textContent = "Здесь появятся назначения, изменения и команды.";
      list.append(empty);
      return;
    }

    for (const action of actions) {
      const row = document.createElement("div");
      row.className = "action-row";
      const main = document.createElement("div");
      main.className = "action-main";
      const actor = document.createElement("strong");
      actor.textContent = action.actorName;
      main.append(actor, document.createTextNode(" " + action.action + " → " + action.targetName + " "));
      const details = document.createElement("span");
      details.className = "action-details";
      details.textContent = action.details;
      main.append(details);
      const time = document.createElement("time");
      time.className = "action-time";
      time.dateTime = action.createdAtUtc;
      time.textContent = new Date(action.createdAtUtc).toLocaleTimeString("ru-RU", {
        hour: "2-digit", minute: "2-digit"
      });
      row.append(main, time);
      list.append(row);
    }
  }

  function makeButton(text, className, callback) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = className;
    button.textContent = text;
    button.addEventListener("click", callback);
    return button;
  }

  async function assignTask(task) {
    const participant = snapshot.participants.find(item => item.id === selectedParticipantId);
    if (!participant) return;
    if (participant.assignedTaskId &&
        !window.confirm("У " + participant.name + " уже есть задание. Назначить «" + task.title + "» вместо него?"))
      return;

    await mutate("/api/assign", {
      roomCode, actorName, participantId: participant.id, taskId: task.id
    });
  }

  async function sendCommand(command) {
    const participant = snapshot.participants.find(item => item.id === selectedParticipantId);
    if (!participant) return;
    if (command === "restart" &&
        !window.confirm("Сбросить позицию печати у " + participant.name + "?"))
      return;

    await mutate("/api/command", {
      roomCode, actorName, participantId: participant.id, command
    });
  }

  function openTaskEditor(task = null) {
    document.querySelector("#taskDialogTitle").textContent = task ? "Изменить задание" : "Новое задание";
    document.querySelector("#taskId").value = task?.id || "";
    document.querySelector("#taskNumber").value = task?.number || "";
    document.querySelector("#taskTitle").value = task?.title || "";
    document.querySelector("#taskLanguage").value = task?.language || "C#";
    document.querySelector("#taskCode").value = task?.code || "";
    taskError.textContent = "";
    taskDialog.showModal();
    document.querySelector("#taskNumber").focus();
  }

  function closeTaskDialog() {
    taskDialog.close();
  }

  async function saveTask(event) {
    event.preventDefault();
    taskError.textContent = "";
    try {
      await request("/api/tasks", {
        method: "POST",
        body: JSON.stringify({
          roomCode,
          actorName,
          id: document.querySelector("#taskId").value || null,
          number: document.querySelector("#taskNumber").value,
          title: document.querySelector("#taskTitle").value,
          language: document.querySelector("#taskLanguage").value,
          code: document.querySelector("#taskCode").value
        })
      });
      closeTaskDialog();
      await refresh();
    } catch (error) {
      taskError.textContent = error.message;
    }
  }

  async function deleteTask(task) {
    if (!window.confirm("Удалить задание №" + task.number + " «" + task.title + "»?")) return;
    try {
      await request(
        "/api/tasks/" + encodeURIComponent(task.id) +
          "?roomCode=" + encodeURIComponent(roomCode) +
          "&actorName=" + encodeURIComponent(actorName),
        { method: "DELETE" });
      await refresh();
    } catch (error) {
      window.alert(error.message);
    }
  }

  async function mutate(path, body) {
    try {
      await request(path, { method: "POST", body: JSON.stringify(body) });
      await refresh();
    } catch (error) {
      window.alert(error.message);
    }
  }
})();
