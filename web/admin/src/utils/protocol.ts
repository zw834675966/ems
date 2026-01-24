import { PROTOCOL_OPTIONS } from "@/config/constants";

/**
 * 根据协议类型生成配置 JSON
 */
export const buildProtocolConfig = (protocolType: string, form: any) => {
  switch (protocolType) {
    case "modbus_tcp":
      return JSON.stringify({
        host: form.modbusHost,
        port: form.modbusPort,
        pollIntervalMs: form.modbusPollInterval
      });
    case "tcp_server":
      return JSON.stringify({
        listenPort: form.tcpServerPort
      });
    case "tcp_client":
      return JSON.stringify({
        host: form.tcpClientHost,
        port: form.tcpClientPort,
        pollIntervalMs: form.tcpClientPollIntervalMs || 1000
      });
    case "mqtt":
    default:
      return undefined;
  }
};

/**
 * 解析协议配置 JSON 到响应式表单对象
 */
export const parseProtocolConfig = (protocolType: string, configStr: string | undefined, form: any) => {
  if (!configStr) return;
  try {
    const config = JSON.parse(configStr);
    switch (protocolType) {
      case "modbus_tcp":
        form.modbusHost = config.host || "";
        form.modbusPort = config.port || 502;
        form.modbusPollInterval = config.pollIntervalMs || config.poll_interval_ms || 1000;
        break;
      case "tcp_server":
        form.tcpServerPort = config.listenPort || config.listen_port || 9000;
        break;
      case "tcp_client":
        form.tcpClientHost = config.host || "";
        form.tcpClientPort = config.port || 8080;
        form.tcpClientPollIntervalMs = config.pollIntervalMs || config.poll_interval_ms || 1000;
        break;
    }
  } catch (e) {
    console.warn("Failed to parse protocol config:", e);
  }
};

/**
 * 获取协议类型显示标签
 */
export const getProtocolLabel = (type: string) => {
  const opt = PROTOCOL_OPTIONS.find(o => o.value === type);
  return opt?.label || type || "MQTT";
};

/**
 * 构建设备地址配置 JSON
 */
export const buildAddressConfig = (protocolType: string | undefined, form: any) => {
  if (protocolType === "modbus_tcp") {
    return JSON.stringify({ unitId: form.unitId });
  }
  if (protocolType === "tcp_server" || protocolType === "tcp_client") {
    if (!form.devId) return undefined;
    const payload: any = { devId: form.devId };
    if (form.devType !== undefined && form.devType !== null && form.devType !== "") {
      payload.devType = form.devType;
    }
    return JSON.stringify(payload);
  }
  return undefined;
};

/**
 * 解析设备地址配置
 */
export const parseAddressConfig = (configStr: string | undefined, form: any) => {
  if (!configStr) return;
  try {
    const config = JSON.parse(configStr);
    form.unitId = config.unitId ?? config.slave_id ?? 1;
    form.devId = config.devId ?? "";
    form.devType = config.devType ?? "";
  } catch (e) {
    console.warn("Failed to parse address config:", e);
  }
};

/**
 * 构建点位协议细节配置 JSON
 */
export const buildProtocolDetail = (protocolType: string | undefined, form: any) => {
  if (protocolType !== "modbus_tcp") return undefined;

  return JSON.stringify({
    functionCode: form.modbusFunctionCode,
    registerAddress: form.modbusRegisterAddress,
    registerCount: form.modbusRegisterCount,
    dataType: form.modbusDataType,
    endian: form.modbusEndian || "big_endian",
    wordOrder: form.modbusWordOrder || undefined
  });
};

/**
 * 解析点位协议细节配置
 */
export const parseProtocolDetail = (detailStr: string | undefined, form: any) => {
  if (!detailStr) return;
  try {
    const detail = JSON.parse(detailStr);
    form.modbusFunctionCode = detail.functionCode ?? detail.function_code ?? 3;
    form.modbusRegisterAddress = detail.registerAddress ?? detail.register_address ?? 0;
    form.modbusRegisterCount = detail.registerCount ?? detail.register_count ?? 1;
    form.modbusDataType = detail.dataType ?? detail.data_type ?? "int16";
    form.modbusEndian = detail.endian ?? detail.byteOrder ?? detail.byte_order ?? "big_endian";
    form.modbusWordOrder = detail.wordOrder ?? detail.word_order ?? "";
  } catch (e) {
    console.warn("Failed to parse protocol detail:", e);
  }
};

export const buildTcpProtocolDetail = (form: any) => {
  if (!form.tcpTag) return undefined;
  return JSON.stringify({
    tag: form.tcpTag,
    valueType: form.tcpValueType || "uint16",
    endian: form.tcpEndian || "big_endian"
  });
};

export const parseTcpProtocolDetail = (detailStr: string | undefined, form: any) => {
  if (!detailStr) return;
  try {
    const detail = JSON.parse(detailStr);
    form.tcpTag = detail.tag ?? "";
    form.tcpValueType = detail.valueType ?? detail.value_type ?? "uint16";
    form.tcpEndian = detail.endian ?? "big_endian";
  } catch (e) {
    console.warn("Failed to parse tcp protocol detail:", e);
  }
};
