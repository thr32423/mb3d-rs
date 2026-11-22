import XCTest

/// The touches the workflow's screenshots cannot show, done as a person
/// would: each test opens the example and checks what the touch changed.
/// Pictures go to the folder in the SCREENS environment value (the
/// workflow's screens folder) and into the test result; while
/// `live(true)`, the workflow takes a picture of the screen every second
/// (what shows only while a finger is down).
final class MB3DUITests: XCTestCase {
    let app = XCUIApplication()
    let example = "ABoxScale2Start"

    override func setUp() {
        continueAfterFailure = true
        XCUIDevice.shared.orientation = .portrait
    }

    override func tearDown() {
        live(false)
        app.terminate()
    }

    // MARK: helpers

    private var folder: URL? {
        ProcessInfo.processInfo.environment["SCREENS"].map { URL(fileURLWithPath: $0) }
    }

    private func shot(_ name: String) {
        let s = XCUIScreen.main.screenshot()
        try? s.pngRepresentation.write(to: folder?.appendingPathComponent("test-\(name).png") ?? URL(fileURLWithPath: "/dev/null"))
        let a = XCTAttachment(screenshot: s)
        a.name = name
        a.lifetime = .keepAlways
        add(a)
    }

    /// The workflow's frames while a finger is down, and a line in
    /// test-phases.txt naming them.
    private func live(_ on: Bool, _ phase: String = "") {
        guard let folder else { return }
        let flag = folder.appendingPathComponent("live-on")
        if on {
            FileManager.default.createFile(atPath: flag.path, contents: Data())
        } else {
            try? FileManager.default.removeItem(at: flag)
        }
        let line = "\(Int(Date().timeIntervalSince1970)) \(on ? "start" : "end") \(phase)\n"
        let log = folder.appendingPathComponent("test-phases.txt")
        if let h = try? FileHandle(forWritingTo: log) {
            h.seekToEndOfFile()
            h.write(Data(line.utf8))
            try? h.close()
        } else {
            try? Data(line.utf8).write(to: log)
        }
    }

    private func open(_ extra: [String] = []) {
        app.launchArguments = ["-open", example] + extra
        app.launch()
    }

    /// the picture of the scene rendered (the status line's size and time)
    private func waitForPicture(_ timeout: TimeInterval = 90) {
        let done = app.staticTexts.matching(NSPredicate(format: "label CONTAINS ' × ' AND label CONTAINS ' s'")).firstMatch
        _ = done.waitForExistence(timeout: timeout)
    }

    private func number(_ title: String) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: "number")
            .matching(NSPredicate(format: "label == %@", title)).firstMatch
    }

    private func scrollTo(_ e: XCUIElement, in list: XCUIElement, tries: Int = 8) {
        var n = 0
        while !(e.exists && e.isHittable) && n < tries {
            list.swipeUp()
            n += 1
        }
    }

    // MARK: the tests

    /// A number dragged sideways changes, a tap types it.
    func test1NumberScrub() {
        open(["-page", "formulas"])
        waitForPicture()
        let scale = number("scale")
        XCTAssertTrue(scale.waitForExistence(timeout: 20), "the formula's scale")
        let before = scale.value as? String
        live(true, "number scrub")
        let start = scale.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
        start.press(forDuration: 0.2, thenDragTo: start.withOffset(CGVector(dx: 220, dy: 0)),
                    withVelocity: .slow, thenHoldForDuration: 4)
        live(false)
        sleep(2)
        let after = scale.value as? String
        XCTAssertNotEqual(before, after, "dragging the number changed it (\(before ?? "?") → \(after ?? "?"))")
        shot("number-scrubbed")
        scale.tap()
        sleep(1)
        shot("number-typing")
        XCTAssertTrue(app.textFields.firstMatch.waitForExistence(timeout: 5), "a tap types the number")
    }

    /// While a slider is held the panels fade and its name and value show
    /// over the picture; the value changes.
    func test2SliderFade() {
        open(["-page", "lighting"])
        waitForPicture()
        let slider = app.sliders.firstMatch
        XCTAssertTrue(slider.waitForExistence(timeout: 20), "the Diffuse slider")
        let before = slider.value as? String
        live(true, "slider held")
        slider.coordinate(withNormalizedOffset: CGVector(dx: 0.25, dy: 0.5))
            .press(forDuration: 0.3, thenDragTo: slider.coordinate(withNormalizedOffset: CGVector(dx: 0.75, dy: 0.5)),
                   withVelocity: .slow, thenHoldForDuration: 6)
        live(false)
        XCTAssertNotEqual(before, slider.value as? String, "the slider moved")
        sleep(15)
        shot("slider-after")
    }

    /// The light ball's dot dragged turns the light.
    func test3LightBall() {
        open(["-page", "lighting"])
        waitForPicture()
        let list = app.collectionViews.firstMatch
        let ball = app.descendants(matching: .any).matching(identifier: "lightball").firstMatch
        _ = list.waitForExistence(timeout: 20)
        scrollTo(ball, in: list)
        XCTAssertTrue(ball.exists, "the light ball")
        let before = ball.value as? String
        shot("lightball-before")
        live(true, "light ball")
        let mid = ball.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.45))
        mid.press(forDuration: 0.2, thenDragTo: mid.withOffset(CGVector(dx: -60, dy: 40)),
                  withVelocity: .slow, thenHoldForDuration: 3)
        live(false)
        sleep(15)
        XCTAssertNotEqual(before, ball.value as? String, "the light turned (\(before ?? "?") → \(ball.value as? String ?? "?"))")
        shot("lightball-after")
    }

    /// One finger turns the camera; a tap with two fingers undoes it, with
    /// three redoes it; a finger held still shows the picture as opened; a
    /// double tap flies to what is there.
    func test4PictureGestures() {
        open()
        waitForPicture()
        let picture = app.descendants(matching: .any).matching(identifier: "picture").firstMatch
        XCTAssertTrue(picture.waitForExistence(timeout: 20), "the picture")
        let undo = app.buttons["Undo"].firstMatch
        let redo = app.buttons["Redo"].firstMatch
        let mid = picture.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
        mid.press(forDuration: 0.2, thenDragTo: mid.withOffset(CGVector(dx: 160, dy: 0)))
        waitForPicture()
        sleep(3)
        shot("gesture-turned")
        XCTAssertTrue(undo.isEnabled, "the turn can be undone")
        picture.tap(withNumberOfTaps: 1, numberOfTouches: 2)
        sleep(4)
        XCTAssertTrue(redo.isEnabled, "a tap with two fingers undid the turn")
        shot("gesture-two-finger-undo")
        picture.tap(withNumberOfTaps: 1, numberOfTouches: 3)
        sleep(4)
        XCTAssertFalse(redo.isEnabled, "a tap with three fingers redid it")
        waitForPicture()
        live(true, "hold to compare")
        mid.press(forDuration: 5)
        live(false)
        picture.doubleTap()
        sleep(2)
        waitForPicture()
        shot("gesture-double-tap-flew")
    }

    /// The folded settings: More on the Lighting and the Effects pages.
    func test5MoreSections() {
        open(["-page", "lighting"])
        waitForPicture()
        let more = app.buttons["More"].firstMatch
        XCTAssertTrue(more.waitForExistence(timeout: 20), "Lighting's More")
        more.tap()
        sleep(2)
        shot("more-lighting")
        app.collectionViews.firstMatch.swipeUp()
        sleep(1)
        shot("more-lighting-2")
        let effects = app.buttons["Effects"].firstMatch
        XCTAssertTrue(effects.waitForExistence(timeout: 5), "the rail's Effects")
        effects.tap()
        sleep(2)
        let list = app.collectionViews.firstMatch
        let mc = app.buttons["Monte carlo rendering"].firstMatch
        scrollTo(mc, in: list)
        XCTAssertTrue(mc.exists, "Effects' Monte carlo rendering")
        mc.tap()
        sleep(2)
        list.swipeUp()
        sleep(1)
        shot("more-monte-carlo")
    }

    /// The formula list with its pictures and descriptions, then a page of
    /// the Formula book.
    func test6FormulaBook() {
        open(["-page", "formulas"])
        waitForPicture()
        let picker = app.descendants(matching: .any).matching(identifier: "formula-picker").firstMatch
        XCTAssertTrue(picker.waitForExistence(timeout: 20), "the formula row")
        picker.tap()
        sleep(20)
        shot("formula-list")
        let about = app.buttons["About this formula..."].firstMatch
        XCTAssertTrue(about.waitForExistence(timeout: 10), "a formula's book button")
        about.tap()
        sleep(20)
        shot("formula-book")
        app.collectionViews.firstMatch.swipeUp()
        sleep(1)
        shot("formula-book-2")
    }

    /// MutaGen's settings in Variations.
    func test7Mutation() {
        open(["-sheet", "variations"])
        let mutation = app.buttons["Mutation"].firstMatch
        XCTAssertTrue(mutation.waitForExistence(timeout: 60), "the Mutation button")
        sleep(30)
        mutation.tap()
        sleep(2)
        shot("mutation")
    }
}
