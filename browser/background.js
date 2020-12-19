// ░▀█▀░█▀█░█▀▄░█▀▀
// ░░█░░█▀█░█▀▄░▀▀█
// ░░▀░░▀░▀░▀▀░░▀▀▀

function listTabs() {
  return browser.tabs.query({})
}

function findTab(props) {
  return browser.tabs.query(props).then((tabs) => tabs[0])
}

function getExtensionInfo() {
  return browser.management.getSelf()
}

// ░█▀▀░█▀█░█▀▀░█░█░█▀▀░▀█▀
// ░▀▀█░█░█░█░░░█▀▄░█▀▀░░█░
// ░▀▀▀░▀▀▀░▀▀▀░▀░▀░▀▀▀░░▀░

const chan = new MessageChannel()
const init = { msg_type: "connect", application: "browser" }

function browserMessage(data) {
  return JSON.stringify({
    msg_type: "browser_message",
    data: data
  })
}

function connect() {
  let ws = new WebSocket('ws://localhost:8080')

  // messages from channel are forwarded to desktopd
  chan.port1.onmessage = (e) => {
    ws.send(e.data)
  }

  // list all tabs and send them to desktopd
  ws.onopen = () => {
    ws.send(JSON.stringify(init))

    listTabs().then((tabs) => {
      ws.send(
        browserMessage({ 
          type: "init",
          data: tabs 
        })
      )
    })
  }

  ws.onmessage = (e) => {
    let cmd = JSON.parse(e.data)
    console.log("Received cmd", cmd)
  }

  ws.onclose = (e) => {
    console.log('Socket is closed. Reconnect will be attempted in 1 second.', e.reason)
    setTimeout(() => {
      connect()
    }, 1000)
  }

  ws.onerror = (err) => {
    console.error('Socket encountered error: ', err.message, 'Closing socket')
    ws.close()
    // attempting to reconnect in a second
    setTimeout(() => {
      connect()
    }, 1000)
  }
}


browser.tabs.onCreated.addListener((tab) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "created",
      data: tab 
    })
  )
})

browser.tabs.onActivated.addListener((o) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "activated",
      tabId: o.tabId, 
      windowId: o.windowId
    })
  )
})

browser.tabs.onAttached.addListener((tabId, o) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "attached",
      tabId: tabId, 
      newWindowId: o.newWindowId,
      newPosition: o.newPosition
    })
  )
})

browser.tabs.onDetached.addListener((tabId, o) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "detached",
      tabId: tabId, 
      oldWindowId: o.oldWindowId,
      oldPosition: o.oldPosition
    })
  )
})

browser.tabs.onHighlighted.addListener((o) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "highlighted",
      windowId: o.windowId,
      tabIds: o.tabIds
    })
  )
})

browser.tabs.onMoved.addListener((tabId,o) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "moved",
      tabId: tabId,
      windowId: o.windowId,
      fromIndex: o.fromIndex,
      toIndex: o.toIndex
    })
  )
})

browser.tabs.onReplaced.addListener((addedTabId,removedTabId) => { 
  chan.port2.postMessage(
    browserMessage({ 
      type: "replaced",
      addedTabId: addedTabId,
      removedTabId: removedTabId
    })
  )
})

browser.tabs.onRemoved.addListener((tabId, o) => { 
  chan.port2.postMessage(
    browserMessage({
      type: "removed",
      tabId: tabId,
      windowId: o.windowId
    })
  )
})

browser.tabs.onUpdated.addListener((_tabId, _info, tab) => { 
  chan.port2.postMessage(
    browserMessage({
      type: "updated",
      data: tab
    })
  )
})

connect()
