use leptos::prelude::*;
use rockbot_ui_model::{BootstrapShellModel, BootstrapStep, FactModel, PanelModel};

#[component]
pub fn BootstrapApp(model: BootstrapShellModel) -> impl IntoView {
    view! {
        <main class="shell" aria-labelledby="app-title">
            <header class="masthead">
                <div class="brand">
                    <div class="brand-lockup">
                        <RockBotLogo />
                        <div class="brand-copy">
                            <p class="eyebrow">{model.hero.eyebrow.clone()}</p>
                            <h1 id="app-title">{model.hero.title.clone()}</h1>
                        </div>
                    </div>
                    <p class="lede">{model.hero.body.clone()}</p>
                </div>

                <nav class="nav-card" aria-label="Planned application surfaces">
                    <p class="nav-card-title">Authenticated surfaces</p>
                    <ul class="nav-list">
                        {model
                            .nav_items
                            .into_iter()
                            .map(|item| view! { <li>{item}</li> })
                            .collect_view()}
                    </ul>
                </nav>
            </header>

            <section class="panel-grid" aria-label="Bootstrap status">
                <Panel panel=model.gateway_panel id_prefix="gateway".to_string() />
            </section>

            <section class="identity-controls panel" aria-labelledby="identity-controls-title">
                <div class="panel-header">
                    <div>
                        <p class="section-eyebrow">Import</p>
                        <h2 id="identity-controls-title">Client identity</h2>
                    </div>
                    <span id="identity-pill" class="pill pill-idle">No key imported</span>
                </div>
                <p class="help">
                    Import a PEM client certificate and PEM private key. RockBot stores the imported
                    identity locally in IndexedDB so the browser can re-authenticate without forcing
                    re-import on every page load.
                </p>

                <div class="auth-banner" aria-live="polite">
                    <div>
                        <p class="section-eyebrow">Connection</p>
                        <strong id="ws-auth-text">Not connected</strong>
                    </div>
                    <span id="auth-surface-pill" class="pill pill-idle">Public bootstrap only</span>
                </div>

                <div id="dropzone" class="dropzone" tabindex="0" role="button" aria-label="Drop PEM files here">
                    <strong>Drop PEM files here</strong>
                    <span>or choose them manually below</span>
                </div>

                <div class="form-grid">
                    <label>
                        <span>Client Certificate (.crt/.pem)</span>
                        <input id="cert-file" type="file" accept=".crt,.pem" />
                    </label>
                    <label>
                        <span>Private Key (.key/.pem)</span>
                        <input id="key-file" type="file" accept=".key,.pem" />
                    </label>
                </div>

                <div class="actions">
                    <button id="save-btn" class="btn btn-primary" type="button">Save Identity</button>
                    <button id="clear-btn" class="btn btn-secondary" type="button">Forget Identity</button>
                </div>

                <pre id="identity-summary" class="summary">No client identity stored.</pre>
            </section>

            <section class="steps panel" aria-labelledby="next-steps-title">
                <div class="panel-header">
                    <div>
                        <p class="section-eyebrow">Flow</p>
                        <h2 id="next-steps-title">Bootstrap sequence</h2>
                    </div>
                    <span class="pill pill-idle">WS-first app</span>
                </div>

                <ol class="step-list">
                    {model
                        .steps
                        .into_iter()
                        .map(|step| view! { <BootstrapStepCard step /> })
                        .collect_view()}
                </ol>
            </section>

            <section id="authenticated-app" class="app-shell hidden" aria-labelledby="workspace-title">
                <div class="panel app-toolbar">
                    <div class="panel-header">
                        <div>
                            <p class="section-eyebrow">Control Plane</p>
                            <h2 id="workspace-title">Agent workspace</h2>
                        </div>
                        <span id="workspace-pill" class="pill pill-idle">Locked</span>
                    </div>
                    <p id="workspace-status" class="help">
                        Authenticate with a client identity to load agent topology, markdown state,
                        and replication controls.
                    </p>
                </div>

                <div class="workspace-grid">
                    <section class="panel agent-sidebar" aria-labelledby="agents-title">
                        <div class="panel-header">
                            <div>
                                <p class="section-eyebrow">Agents</p>
                                <h3 id="agents-title">Create and select</h3>
                            </div>
                            <span id="agent-count-pill" class="pill pill-idle">0 agents</span>
                        </div>

                        <form id="agent-create-form" class="stack-form">
                            <label>
                                <span>Agent ID</span>
                                <input id="agent-create-id" type="text" placeholder="hex-worker" />
                            </label>
                            <label>
                                <span>Model</span>
                                <input id="agent-create-model" type="text" placeholder="bedrock/..." />
                            </label>
                            <label>
                                <span>Owner Agent</span>
                                <input id="agent-create-owner" type="text" placeholder="Hex" />
                            </label>
                            <label>
                                <span>Zone</span>
                                <input id="agent-create-zone" type="text" placeholder="zone:hex" />
                            </label>
                            <label>
                                <span>System Prompt</span>
                                <textarea id="agent-create-prompt" rows="6" placeholder="Describe the agent's role and constraints."></textarea>
                            </label>
                            <button class="btn btn-primary" type="submit">Create Agent</button>
                        </form>

                        <div id="agent-list" class="agent-list" aria-live="polite"></div>
                    </section>

                    <section class="panel agent-detail" aria-labelledby="agent-detail-title">
                        <div class="panel-header">
                            <div>
                                <p class="section-eyebrow">Agent</p>
                                <h3 id="agent-detail-title">No agent selected</h3>
                            </div>
                            <span id="selected-agent-pill" class="pill pill-idle">Idle</span>
                        </div>

                        <form id="agent-settings-form" class="settings-grid">
                            <label>
                                <span>Owner Agent</span>
                                <input id="agent-owner-input" type="text" />
                            </label>
                            <label>
                                <span>Zone</span>
                                <input id="agent-zone-input" type="text" />
                            </label>
                            <label>
                                <span>Model</span>
                                <input id="agent-model-input" type="text" />
                            </label>
                            <label>
                                <span>Parent Agent</span>
                                <input id="agent-parent-input" type="text" />
                            </label>
                            <button id="agent-settings-save" class="btn btn-secondary" type="submit">Save Agent Settings</button>
                        </form>

                        <div class="meta-strip">
                            <div class="meta-chip">
                                <strong>Creator</strong>
                                <span id="agent-creator-value">-</span>
                            </div>
                            <div class="meta-chip">
                                <strong>Workspace</strong>
                                <span id="agent-vdisk-value">-</span>
                            </div>
                            <div class="meta-chip">
                                <strong>Status</strong>
                                <span id="agent-status-value">-</span>
                            </div>
                        </div>

                        <div class="editor-shell">
                            <aside class="doc-list-panel">
                                <div class="panel-header compact">
                                    <div>
                                        <p class="section-eyebrow">Documents</p>
                                        <h4>Markdown state</h4>
                                    </div>
                                    <span id="doc-pill" class="pill pill-idle">No document</span>
                                </div>
                                <div id="doc-list" class="doc-list"></div>
                            </aside>

                            <div class="doc-editor-panel">
                                <div class="panel-header compact">
                                    <div>
                                        <p class="section-eyebrow">Editor</p>
                                        <h4 id="doc-editor-title">Select a markdown document</h4>
                                    </div>
                                    <button id="doc-save-btn" class="btn btn-primary" type="button">Save Markdown</button>
                                </div>
                                <textarea id="doc-editor" class="doc-editor" spellcheck="false"></textarea>
                            </div>
                        </div>
                    </section>
                </div>

                <div class="workspace-grid lower">
                    <section class="panel replication-panel" aria-labelledby="replication-title">
                        <div class="panel-header">
                            <div>
                                <p class="section-eyebrow">Replication</p>
                                <h3 id="replication-title">Large object policy</h3>
                            </div>
                            <span id="replication-pill" class="pill pill-idle">No objects</span>
                        </div>
                        <p class="help">
                            Promote large objects for replication selectively. Canonical markdown documents remain
                            replicated through the agent state store; this table is for larger attached objects.
                        </p>
                        <div id="object-list" class="object-list"></div>
                    </section>

                    <section class="panel topology-panel" aria-labelledby="topology-title">
                        <div class="panel-header">
                            <div>
                                <p class="section-eyebrow">Topology</p>
                                <h3 id="topology-title">Zones and communication graph</h3>
                            </div>
                            <span id="topology-pill" class="pill pill-idle">No topology</span>
                        </div>
                        <div class="topology-columns">
                            <div>
                                <h4>Nodes</h4>
                                <div id="topology-nodes" class="topology-list"></div>
                            </div>
                            <div>
                                <h4>Edges</h4>
                                <div id="topology-edges" class="topology-list"></div>
                            </div>
                            <div>
                                <h4>Zones</h4>
                                <div id="topology-zones" class="topology-list"></div>
                            </div>
                        </div>
                    </section>
                </div>
            </section>
        </main>
    }
}

#[component]
fn Panel(panel: PanelModel, id_prefix: String) -> impl IntoView {
    let heading_id = format!("{id_prefix}-heading");
    let status_id = format!("{id_prefix}-status");
    let heading_for_section = heading_id.clone();
    let heading_for_title = heading_id.clone();

    view! {
        <section class="panel" aria-labelledby=heading_for_section>
            <div class="panel-header">
                <div>
                    <p class="section-eyebrow">Status</p>
                    <h2 id=heading_for_title>{panel.title}</h2>
                </div>
                <span id=status_id class=format!("pill {}", panel.pill.tone.css_class())>
                    {panel.pill.label}
                </span>
            </div>
            {panel
                .description
                .map(|description| view! { <p class="help">{description}</p> })}
            <dl class="facts">
                {panel
                    .facts
                    .into_iter()
                    .enumerate()
                    .map(|(index, fact)| {
                        view! { <FactCard fact id_prefix=format!("{id_prefix}-{index}") /> }
                    })
                    .collect_view()}
            </dl>
        </section>
    }
}

#[component]
fn FactCard(fact: FactModel, id_prefix: String) -> impl IntoView {
    let label_id = format!("{id_prefix}-label");
    let value_id = format!("{id_prefix}-value");
    let label_for_card = label_id.clone();
    let label_for_term = label_id.clone();
    let value_for_card = value_id.clone();
    let value_for_desc = value_id.clone();
    let is_health = fact.label == "Health";
    let is_ws_auth = fact.label == "WS Auth";

    view! {
        <div class="fact-card" aria-labelledby=label_for_card aria-describedby=value_for_card>
            <dt id=label_for_term>{fact.label.clone()}</dt>
            <dd id=value_for_desc>
                {match fact.href {
                    Some(href) => view! { <a href=href rel="noreferrer">{fact.value}</a> }.into_any(),
                    None if is_health => view! { <span id="health-text">{fact.value}</span> }.into_any(),
                    None if is_ws_auth => view! { <span id="ws-auth-text">{fact.value}</span> }.into_any(),
                    None => view! { <span>{fact.value}</span> }.into_any(),
                }}
            </dd>
        </div>
    }
}

#[component]
fn BootstrapStepCard(step: BootstrapStep) -> impl IntoView {
    view! {
        <li class="step-card">
            <h3>{step.title}</h3>
            <p>{step.body}</p>
        </li>
    }
}

#[component]
fn RockBotLogo() -> impl IntoView {
    view! {
        <svg
            class="rockbot-logo"
            viewBox="0 0 84 84"
            role="img"
            aria-label="RockBot logo"
        >
            <defs>
                <linearGradient id="rockbot-logo-fill" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stop-color="#2dd4bf"></stop>
                    <stop offset="100%" stop-color="#38bdf8"></stop>
                </linearGradient>
            </defs>
            <rect x="6" y="6" width="72" height="72" rx="20" fill="rgba(8, 17, 31, 0.72)"></rect>
            <path
                d="M24 58V26h18c10.2 0 16 5.3 16 13.4 0 6.2-3.5 10.3-9.5 12L61 58H48.4l-10-10.1H35.7V58H24zm11.7-18.9h5.6c3.6 0 5.5-1.6 5.5-4.3s-1.9-4.2-5.5-4.2h-5.6v8.5z"
                fill="url(#rockbot-logo-fill)"
            ></path>
        </svg>
    }
}
