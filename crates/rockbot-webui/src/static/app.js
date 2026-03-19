const DB_NAME = 'rockbot-web-bootstrap';
const STORE_NAME = 'identity';
const ID_KEY = 'client-identity';

let activeSocket = null;
let socketAuthenticated = false;
let activeIdentity = null;
let nextRequestId = 1;
const pendingApiRequests = new Map();

const appState = {
  agents: [],
  selectedAgentId: null,
  files: [],
  selectedFileName: null,
  objects: [],
  topology: { nodes: [], edges: [], zones: [] },
};

function el(id) {
  return document.getElementById(id);
}

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function setPill(id, cls, text) {
  const node = el(id);
  if (!node) return;
  node.className = `pill ${cls}`;
  node.textContent = text;
}

function setText(id, text) {
  const node = el(id);
  if (node) node.textContent = text;
}

function renderEmpty(node, text) {
  if (!node) return;
  node.innerHTML = `<div class="empty-state">${escapeHtml(text)}</div>`;
}

function openDb() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function idbGet(key) {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readonly');
    const store = tx.objectStore(STORE_NAME);
    const request = store.get(key);
    request.onsuccess = () => resolve(request.result ?? null);
    request.onerror = () => reject(request.error);
  });
}

async function idbSet(key, value) {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).put(value, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function idbDelete(key) {
  const db = await openDb();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).delete(key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

function summarizeIdentity(identity) {
  if (!identity) {
    return 'No client identity stored.';
  }
  const certPreview = identity.certificate.split('\n').slice(0, 3).join('\n');
  return [
    `Stored at: ${new Date(identity.savedAt).toLocaleString()}`,
    '',
    'Certificate preview:',
    certPreview,
    '',
    `Private key: ${identity.privateKey instanceof CryptoKey ? 'stored as non-extractable CryptoKey' : 'legacy PEM'}`,
  ].join('\n');
}

function updateWsAuth(text, cls) {
  setText('ws-auth-text', text);
  setPill(
    'identity-pill',
    cls,
    cls === 'pill-ok' ? 'Identity ready' : (el('identity-pill')?.textContent || text),
  );
  setPill(
    'auth-surface-pill',
    cls,
    cls === 'pill-ok' ? 'Authenticated surface ready' : 'Public bootstrap only',
  );
}

function setWorkspaceVisible(visible) {
  const app = el('authenticated-app');
  if (!app) return;
  app.classList.toggle('hidden', !visible);
  setPill('workspace-pill', visible ? 'pill-ok' : 'pill-idle', visible ? 'Live' : 'Locked');
  setText(
    'workspace-status',
    visible
      ? 'Authenticated WebSocket connected. Agent management and canonical markdown storage are live.'
      : 'Authenticate with a client identity to load agent topology, markdown state, and replication controls.',
  );
}

async function refreshIdentity() {
  const identity = await idbGet(ID_KEY);
  activeIdentity = identity;
  setPill('identity-pill', identity ? 'pill-ok' : 'pill-idle', identity ? 'Identity ready' : 'No key imported');
  setText('identity-summary', summarizeIdentity(identity));
  if (identity) {
    void ensureAuthenticatedSocket(identity);
  } else {
    updateWsAuth('No stored identity', 'pill-idle');
    setWorkspaceVisible(false);
  }
}

async function refreshHealth() {
  try {
    const response = await fetch('/health');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const health = await response.json();
    setPill('gateway-status', 'pill-ok', 'Healthy');
    setText('health-text', `${health.status || 'ok'} · v${health.version || 'unknown'}`);
  } catch (error) {
    setPill('gateway-status', 'pill-warn', 'Unavailable');
    setText('health-text', String(error));
  }
}

async function readFileInput(input) {
  const file = input.files && input.files[0];
  if (!file) return null;
  return file.text();
}

function pemBody(pem) {
  return pem
    .replace(/-----BEGIN [^-]+-----/g, '')
    .replace(/-----END [^-]+-----/g, '')
    .replace(/\s+/g, '');
}

function base64ToArrayBuffer(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

function arrayBufferToBase64(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary);
}

async function importPrivateKey(privateKeyPem) {
  return window.crypto.subtle.importKey(
    'pkcs8',
    base64ToArrayBuffer(pemBody(privateKeyPem)),
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['sign'],
  );
}

async function signChallengeWithKey(privateKey, challengeBase64) {
  const signature = await window.crypto.subtle.sign(
    { name: 'ECDSA', hash: 'SHA-256' },
    privateKey,
    base64ToArrayBuffer(challengeBase64),
  );
  return arrayBufferToBase64(signature);
}

function rejectPendingRequests(message) {
  for (const [, handlers] of pendingApiRequests) {
    handlers.reject(new Error(message));
  }
  pendingApiRequests.clear();
}

async function apiRequest(method, path, body) {
  if (!activeSocket || activeSocket.readyState !== WebSocket.OPEN || !socketAuthenticated) {
    throw new Error('Authenticated WebSocket not ready');
  }
  const requestId = `web-${nextRequestId++}`;
  const payload = {
    type: 'api_request',
    request_id: requestId,
    method,
    path,
  };
  if (body !== undefined) {
    payload.body = body;
  }
  const promise = new Promise((resolve, reject) => {
    pendingApiRequests.set(requestId, { resolve, reject });
  });
  activeSocket.send(JSON.stringify(payload));
  const response = await promise;
  const trimmedBody = typeof response.body === 'string' ? response.body.trim() : '';
  const parsedBody = trimmedBody && (trimmedBody.startsWith('{') || trimmedBody.startsWith('['))
    ? JSON.parse(trimmedBody)
    : response.body;
  if (response.status >= 400) {
    const message = typeof parsedBody === 'string'
      ? parsedBody
      : parsedBody.error || parsedBody.message || `HTTP ${response.status}`;
    throw new Error(message);
  }
  return parsedBody;
}

function handleApiResponse(payload) {
  const pending = pendingApiRequests.get(payload.request_id);
  if (!pending) return;
  pendingApiRequests.delete(payload.request_id);
  pending.resolve(payload);
}

async function saveIdentity() {
  try {
    const certPem = await readFileInput(el('cert-file'));
    const keyPem = await readFileInput(el('key-file'));
    if (!certPem || !keyPem) {
      setPill('identity-pill', 'pill-warn', 'Need cert + key');
      return;
    }
    const privateKey = await importPrivateKey(keyPem);
    await idbSet(ID_KEY, {
      certificate: certPem,
      privateKey,
      savedAt: Date.now(),
    });
    await refreshIdentity();
  } catch (error) {
    updateWsAuth(`Identity import failed: ${error}`, 'pill-warn');
  }
}

async function clearIdentity() {
  if (activeSocket) {
    activeSocket.close();
    activeSocket = null;
  }
  socketAuthenticated = false;
  await idbDelete(ID_KEY);
  if (el('cert-file')) el('cert-file').value = '';
  if (el('key-file')) el('key-file').value = '';
  updateWsAuth('Not connected', 'pill-idle');
  setWorkspaceVisible(false);
  rejectPendingRequests('Identity cleared');
  await refreshIdentity();
}

async function ensureAuthenticatedSocket(identity) {
  if (activeSocket && activeSocket.readyState === WebSocket.OPEN && socketAuthenticated) {
    return;
  }

  if (activeSocket && activeSocket.readyState === WebSocket.OPEN) {
    activeSocket.close();
  }

  const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
  const socket = new WebSocket(`${scheme}://${window.location.host}/ws`);
  activeSocket = socket;
  socketAuthenticated = false;
  updateWsAuth('Connecting...', 'pill-idle');

  socket.addEventListener('open', () => {
    socket.send(JSON.stringify({
      type: 'web_auth_begin',
      certificate_pem: identity.certificate,
    }));
  });

  socket.addEventListener('message', async (event) => {
    const payload = JSON.parse(event.data);
    if (payload.type === 'web_auth_challenge') {
      try {
        const signature = await signChallengeWithKey(identity.privateKey, payload.challenge);
        socket.send(JSON.stringify({
          type: 'web_auth_complete',
          signature,
        }));
      } catch (error) {
        updateWsAuth(`Key import failed: ${error}`, 'pill-warn');
      }
      return;
    }

    if (payload.type === 'web_auth_result') {
      if (payload.authenticated) {
        socketAuthenticated = true;
        updateWsAuth(`Authenticated as ${payload.cert_name} (${payload.cert_role})`, 'pill-ok');
        setWorkspaceVisible(true);
        try {
          await loadAuthenticatedSurface();
        } catch (error) {
          setText('workspace-status', `Authenticated, but the management surface failed to load: ${error}`);
        }
      } else {
        socketAuthenticated = false;
        setWorkspaceVisible(false);
        updateWsAuth(payload.message || 'Authentication failed', 'pill-warn');
      }
      return;
    }

    if (payload.type === 'api_response') {
      handleApiResponse(payload);
    }
  });

  socket.addEventListener('close', () => {
    if (activeSocket === socket) {
      activeSocket = null;
      socketAuthenticated = false;
      setWorkspaceVisible(false);
      updateWsAuth('Disconnected', 'pill-warn');
      rejectPendingRequests('WebSocket closed');
    }
  });

  socket.addEventListener('error', () => {
    updateWsAuth('WebSocket error', 'pill-warn');
  });
}

function bindDropzone() {
  const dropzone = el('dropzone');
  if (!dropzone) return;

  const prevent = (event) => {
    event.preventDefault();
    event.stopPropagation();
  };

  ['dragenter', 'dragover'].forEach((name) => {
    dropzone.addEventListener(name, (event) => {
      prevent(event);
      dropzone.classList.add('active');
    });
  });

  ['dragleave', 'drop'].forEach((name) => {
    dropzone.addEventListener(name, (event) => {
      prevent(event);
      dropzone.classList.remove('active');
    });
  });

  dropzone.addEventListener('drop', async (event) => {
    const files = Array.from(event.dataTransfer?.files || []);
    const existing = await idbGet(ID_KEY) || {};
    const updated = { ...existing, savedAt: Date.now() };
    for (const file of files) {
      const text = await file.text();
      if (text.includes('BEGIN CERTIFICATE')) {
        updated.certificate = text;
      } else if (text.includes('BEGIN') && text.includes('PRIVATE KEY')) {
        updated.privateKey = await importPrivateKey(text);
      }
    }
    await idbSet(ID_KEY, updated);
    await refreshIdentity();
  });
}

function selectedAgent() {
  return appState.agents.find((agent) => agent.id === appState.selectedAgentId) || null;
}

async function loadAuthenticatedSurface() {
  setText('workspace-status', 'Loading authenticated agent workspace...');
  await Promise.all([refreshAgents(), refreshTopology()]);
  setText('workspace-status', 'Authenticated agent workspace loaded.');
}

async function refreshAgents() {
  const agents = await apiRequest('GET', '/api/agents');
  appState.agents = Array.isArray(agents) ? agents : [];
  if (!appState.selectedAgentId || !appState.agents.some((agent) => agent.id === appState.selectedAgentId)) {
    appState.selectedAgentId = appState.agents[0]?.id || null;
  }
  renderAgentList();
  if (appState.selectedAgentId) {
    await refreshSelectedAgent();
  } else {
    clearSelectedAgentSurface();
  }
}

async function refreshSelectedAgent() {
  const agent = selectedAgent();
  if (!agent) {
    clearSelectedAgentSurface();
    return;
  }
  setText('agent-detail-title', agent.id);
  setPill('selected-agent-pill', agent.enabled === false ? 'pill-warn' : 'pill-ok', agent.status || 'configured');
  setText('agent-creator-value', agent.creator_agent_id || '-');
  setText('agent-vdisk-value', `${agent.id}.data`);
  setText('agent-status-value', agent.status || 'configured');
  if (el('agent-owner-input')) el('agent-owner-input').value = agent.owner_agent_id || '';
  if (el('agent-zone-input')) el('agent-zone-input').value = agent.zone_id || '';
  if (el('agent-model-input')) el('agent-model-input').value = agent.model || '';
  if (el('agent-parent-input')) el('agent-parent-input').value = agent.parent_id || '';

  await Promise.all([refreshFiles(agent.id), refreshObjects(agent.id), refreshTopology()]);
}

function clearSelectedAgentSurface() {
  setText('agent-detail-title', 'No agent selected');
  setPill('selected-agent-pill', 'pill-idle', 'Idle');
  setText('agent-creator-value', '-');
  setText('agent-vdisk-value', '-');
  setText('agent-status-value', '-');
  if (el('doc-editor')) el('doc-editor').value = '';
  renderEmpty(el('doc-list'), 'Create an agent to start editing canonical markdown documents.');
  renderEmpty(el('object-list'), 'No agent selected.');
}

function renderAgentList() {
  setPill('agent-count-pill', appState.agents.length ? 'pill-ok' : 'pill-idle', `${appState.agents.length} agent${appState.agents.length === 1 ? '' : 's'}`);
  const node = el('agent-list');
  if (!node) return;
  if (!appState.agents.length) {
    renderEmpty(node, 'No agents exist yet. Create one to seed its per-agent vdisk and topology state.');
    return;
  }

  node.innerHTML = appState.agents.map((agent) => `
    <article class="agent-card ${agent.id === appState.selectedAgentId ? 'active' : ''}">
      <div class="agent-card-title">${escapeHtml(agent.id)}</div>
      <p class="agent-card-meta">Model: ${escapeHtml(agent.model || 'unconfigured')}</p>
      <p class="agent-card-meta">Owner: ${escapeHtml(agent.owner_agent_id || '-')} · Zone: ${escapeHtml(agent.zone_id || '-')}</p>
      <button class="btn btn-secondary" type="button" data-select-agent="${escapeHtml(agent.id)}">Manage agent</button>
    </article>
  `).join('');

  node.querySelectorAll('[data-select-agent]').forEach((button) => {
    button.addEventListener('click', async () => {
      appState.selectedAgentId = button.getAttribute('data-select-agent');
      renderAgentList();
      await refreshSelectedAgent();
    });
  });
}

async function refreshFiles(agentId) {
  const files = await apiRequest('GET', `/api/agents/${encodeURIComponent(agentId)}/files`);
  appState.files = Array.isArray(files) ? files : [];
  if (!appState.selectedFileName || !appState.files.some((file) => file.name === appState.selectedFileName)) {
    appState.selectedFileName = appState.files[0]?.name || null;
  }
  renderFileList();
  if (appState.selectedFileName) {
    await loadFile(agentId, appState.selectedFileName);
  } else if (el('doc-editor')) {
    el('doc-editor').value = '';
  }
}

function renderFileList() {
  const node = el('doc-list');
  if (!node) return;
  if (!appState.files.length) {
    renderEmpty(node, 'No canonical markdown documents were found for this agent.');
    setPill('doc-pill', 'pill-idle', 'No document');
    return;
  }

  setPill('doc-pill', 'pill-ok', `${appState.files.length} docs`);
  node.innerHTML = appState.files.map((file) => `
    <article class="doc-card ${file.name === appState.selectedFileName ? 'active' : ''}">
      <div class="doc-card-title">${escapeHtml(file.name)}</div>
      <p class="doc-card-meta">${file.well_known ? 'Canonical' : 'Custom'} · ${file.size_bytes || 0} bytes</p>
      <button class="btn btn-secondary" type="button" data-select-doc="${escapeHtml(file.name)}">Open document</button>
    </article>
  `).join('');

  node.querySelectorAll('[data-select-doc]').forEach((button) => {
    button.addEventListener('click', async () => {
      const fileName = button.getAttribute('data-select-doc');
      appState.selectedFileName = fileName;
      renderFileList();
      if (appState.selectedAgentId && fileName) {
        await loadFile(appState.selectedAgentId, fileName);
      }
    });
  });
}

async function loadFile(agentId, fileName) {
  const payload = await apiRequest('GET', `/api/agents/${encodeURIComponent(agentId)}/files/${encodeURIComponent(fileName)}`);
  setText('doc-editor-title', fileName);
  if (el('doc-editor')) el('doc-editor').value = payload.content || '';
}

async function saveCurrentDocument() {
  const agent = selectedAgent();
  if (!agent || !appState.selectedFileName) return;
  const content = el('doc-editor')?.value || '';
  await apiRequest('PUT', `/api/agents/${encodeURIComponent(agent.id)}/files/${encodeURIComponent(appState.selectedFileName)}`, {
    content,
  });
  setText('workspace-status', `Saved ${appState.selectedFileName} for ${agent.id}.`);
  await refreshFiles(agent.id);
}

async function refreshObjects(agentId) {
  const objects = await apiRequest('GET', `/api/agents/${encodeURIComponent(agentId)}/objects`);
  appState.objects = Array.isArray(objects) ? objects : [];
  renderObjectList();
}

function renderObjectList() {
  const node = el('object-list');
  if (!node) return;
  if (!appState.selectedAgentId) {
    renderEmpty(node, 'Select an agent to inspect large-object replication policy.');
    setPill('replication-pill', 'pill-idle', 'No objects');
    return;
  }
  if (!appState.objects.length) {
    renderEmpty(node, 'No large objects are tracked for this agent yet.');
    setPill('replication-pill', 'pill-idle', 'No objects');
    return;
  }

  setPill('replication-pill', 'pill-ok', `${appState.objects.length} object${appState.objects.length === 1 ? '' : 's'}`);
  node.innerHTML = appState.objects.map((object) => `
    <article class="object-card">
      <div class="object-row">
        <div>
          <strong>${escapeHtml(object.object_id)}</strong>
          <p class="object-card-meta">${escapeHtml(object.content_type)} · ${object.size_bytes || 0} bytes</p>
          <p class="object-card-meta">Hash: ${escapeHtml(object.hash || '-')}</p>
        </div>
        <div class="object-controls">
          <select data-object-class="${escapeHtml(object.object_id)}">
            ${['required', 'preferred', 'local_only', 'manual_promote'].map((value) => `
              <option value="${value}" ${value === object.replication_class ? 'selected' : ''}>${value}</option>
            `).join('')}
          </select>
          <label class="checkbox-line">
            <input type="checkbox" data-object-promote="${escapeHtml(object.object_id)}" ${object.promoted_for_replication ? 'checked' : ''} />
            <span>Replicate</span>
          </label>
        </div>
        <button class="btn btn-secondary" type="button" data-save-object="${escapeHtml(object.object_id)}">Update</button>
      </div>
    </article>
  `).join('');

  node.querySelectorAll('[data-save-object]').forEach((button) => {
    button.addEventListener('click', async () => {
      const objectId = button.getAttribute('data-save-object');
      const replicationClass = node.querySelector(`[data-object-class="${CSS.escape(objectId)}"]`)?.value;
      const promoted = node.querySelector(`[data-object-promote="${CSS.escape(objectId)}"]`)?.checked;
      if (!appState.selectedAgentId || !objectId) return;
      await apiRequest('PUT', `/api/agents/${encodeURIComponent(appState.selectedAgentId)}/objects/${encodeURIComponent(objectId)}`, {
        replication_class: replicationClass,
        promoted_for_replication: Boolean(promoted),
      });
      setText('workspace-status', `Updated replication policy for ${objectId}.`);
      await refreshObjects(appState.selectedAgentId);
    });
  });
}

async function refreshTopology() {
  const topology = await apiRequest('GET', '/api/topology');
  appState.topology = topology || { nodes: [], edges: [], zones: [] };
  renderTopology();
}

function renderTopology() {
  const nodesNode = el('topology-nodes');
  const edgesNode = el('topology-edges');
  const zonesNode = el('topology-zones');
  const nodes = Array.isArray(appState.topology.nodes) ? appState.topology.nodes : [];
  const edges = Array.isArray(appState.topology.edges) ? appState.topology.edges : [];
  const zones = Array.isArray(appState.topology.zones) ? appState.topology.zones : [];
  setPill('topology-pill', nodes.length ? 'pill-ok' : 'pill-idle', `${nodes.length} node${nodes.length === 1 ? '' : 's'}`);

  if (nodesNode) {
    nodesNode.innerHTML = nodes.length ? nodes.map((node) => `
      <article class="topology-card">
        <div class="topology-card-title">${escapeHtml(node.agent_id)}</div>
        <p class="topology-card-meta">Creator: ${escapeHtml(node.creator_agent_id || '-')}</p>
        <p class="topology-card-meta">Owner: ${escapeHtml(node.owner_agent_id || '-')} · Zone: ${escapeHtml(node.zone_id || '-')}</p>
      </article>
    `).join('') : '<div class="empty-state">No topology nodes recorded yet.</div>';
  }

  if (edgesNode) {
    edgesNode.innerHTML = edges.length ? edges.map((edge) => `
      <article class="topology-card">
        <div class="topology-card-title">${escapeHtml(edge.edge_kind)}</div>
        <p class="topology-card-meta">${escapeHtml(edge.from_agent_id)} → ${escapeHtml(edge.to_agent_id)}</p>
      </article>
    `).join('') : '<div class="empty-state">No communication edges recorded yet.</div>';
  }

  if (zonesNode) {
    zonesNode.innerHTML = zones.length ? zones.map((zone) => `
      <article class="zone-card">
        <div class="topology-card-title">${escapeHtml(zone.zone_id)}</div>
        <p class="zone-card-meta">Owner: ${escapeHtml(zone.owner_agent_id || '-')} · Max agents: ${zone.max_agents}</p>
        <p class="zone-card-meta">Cross-zone delegation: ${zone.allow_cross_zone_delegation ? 'enabled' : 'blocked'}</p>
      </article>
    `).join('') : '<div class="empty-state">No zones recorded yet.</div>';
  }
}

async function handleCreateAgent(event) {
  event.preventDefault();
  const id = el('agent-create-id')?.value.trim();
  if (!id) {
    setText('workspace-status', 'Agent ID is required.');
    return;
  }
  await apiRequest('POST', '/api/agents', {
    id,
    model: el('agent-create-model')?.value.trim() || undefined,
    owner_agent_id: el('agent-create-owner')?.value.trim() || undefined,
    zone_id: el('agent-create-zone')?.value.trim() || undefined,
    system_prompt: el('agent-create-prompt')?.value || undefined,
  });
  event.target.reset();
  appState.selectedAgentId = id;
  setText('workspace-status', `Created agent ${id}.`);
  await refreshAgents();
}

async function handleSaveAgentSettings(event) {
  event.preventDefault();
  const agent = selectedAgent();
  if (!agent) return;
  await apiRequest('PUT', `/api/agents/${encodeURIComponent(agent.id)}`, {
    owner_agent_id: el('agent-owner-input')?.value.trim() || '',
    zone_id: el('agent-zone-input')?.value.trim() || '',
    model: el('agent-model-input')?.value.trim() || '',
    parent_id: el('agent-parent-input')?.value.trim() || '',
  });
  setText('workspace-status', `Saved settings for ${agent.id}.`);
  await refreshAgents();
}

function bindAppActions() {
  el('save-btn')?.addEventListener('click', () => { void saveIdentity(); });
  el('clear-btn')?.addEventListener('click', () => { void clearIdentity(); });
  el('doc-save-btn')?.addEventListener('click', () => { void saveCurrentDocument(); });
  el('agent-create-form')?.addEventListener('submit', (event) => { void handleCreateAgent(event); });
  el('agent-settings-form')?.addEventListener('submit', (event) => { void handleSaveAgentSettings(event); });
}

window.addEventListener('DOMContentLoaded', async () => {
  bindDropzone();
  bindAppActions();
  await refreshHealth();
  await refreshIdentity();
});
