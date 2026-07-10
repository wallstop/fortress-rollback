extends Node

const PROBE_CLASS := &"FortressEmscriptenProbe"
const RESULT_KEY := "__FORTRESS_ROLLBACK_RESULT__"


func _ready() -> void:
	var result := _run_probe()
	result["godot_threads"] = OS.has_feature("threads")
	result["godot_version"] = Engine.get_version_info().get("string", "unknown")
	_publish(result)


func _run_probe() -> Dictionary:
	if not ClassDB.class_exists(PROBE_CLASS):
		return _error_result("Rust GDExtension class is not registered")
	if not ClassDB.can_instantiate(PROBE_CLASS):
		return _error_result("Rust GDExtension class cannot be instantiated")

	var probe: Object = ClassDB.instantiate(PROBE_CLASS)
	if probe == null:
		return _error_result("Rust GDExtension class instantiation returned null")

	var result: Variant = probe.call("run_probe")
	if not result is Dictionary:
		return _error_result("Rust GDExtension returned a non-Dictionary result")
	return result


func _error_result(message: String) -> Dictionary:
	return {
		"status": "complete",
		"ok": false,
		"mode": "unknown",
		"target_os": "unknown",
		"real_clock_smoke": false,
		"real_clock_send_delta": 0,
		"ping_a_ms": -1,
		"ping_b_ms": -1,
		"error": message,
	}


func _publish(result: Dictionary) -> void:
	result["status"] = "complete"
	var payload := JSON.stringify(result)
	JavaScriptBridge.eval("globalThis.%s = %s" % [RESULT_KEY, payload], true)
	print("FORTRESS_EMSCRIPTEN_RESULT ", payload)
