const Desktopd = {
  foo: "bar",
  init: function () {
    console.log("initializing desktopd")

    const STORAGE_KEY = "desktopd-browser-id"
    const chan = new MessageChannel()
    const url = 'ws://localhost:8080'
    var ws;

    function getId() {
      return window.localStorage.getItem(STORAGE_KEY)
    }

    function listTabs() {
      return browser.tabs.query({})
    }

    function findTab(props) {
      return browser.tabs.query(props).then((tabs) => tabs[0])
    }

    function getExtensionInfo() {
      return browser.management.getSelf()
    }

    function handleCliRequest(cmd) {
      switch (cmd.cli_request) {
        case 'focus_tab':
          browser.tabs.update(cmd.tabId, {
            active: true
          }).then((e) =>
            console.log("activated tab", cmd.tabId, e)
          )
      }
    }

    function handleCommand(cmd) {
      switch (cmd.msg_type) {
        case 'cli_request':
          handleCliRequest(cmd)
          break
        default:
          console.log('unhandled command', cmd)
      }
    }

    function browserMessage(data) {
      return JSON.stringify({
        msg_type: "browser_message",
        data: data
      })
    }

    function initMessage() {
      return {
        msg_type: "connect",
        application: "browser",
        id: getId()
      }
    }

    function connect() {
      console.log("connecting to", url)

      if (ws != null && ws != undefined) {
        console.log("cleaning up previous connection")
        ws.close()
        delete ws
      }

      ws = new WebSocket(url)

      // list all tabs and send them to desktopd
      ws.onopen = () => {
        ws.send(JSON.stringify(initMessage()))

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
        handleCommand(cmd)
      }

      ws.onclose = (e) => {
        console.log('Socket is closed. Reconnect will be attempted in 1 second.', e.reason)
        ws.close()
        setTimeout(() => {
          connect()
        }, 1000)
      }

      ws.onerror = (err) => {
        console.error('Socket encountered error: ', err, 'Closing socket')
        ws.close()
        // attempting to reconnect in a second
        setTimeout(() => {
          connect()
        }, 1000)
      }

      // messages from channel are forwarded to desktopd
      chan.port1.onmessage = (e) => {
        ws.send(e.data)
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

    browser.tabs.onMoved.addListener((tabId, o) => {
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

    browser.tabs.onReplaced.addListener((addedTabId, removedTabId) => {
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

    console.log("checking instance id")
    if (window.localStorage.getItem(STORAGE_KEY) == null) {
      let id = browser.runtime.id;
      window.localStorage.setItem(STORAGE_KEY, id)
    }

    console.log("browser id is: ", getId())
    console.log("connecting to daemon")

    connect()
  }
}

Desktopd.init()
