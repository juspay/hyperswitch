var forEach = function (obj, callback) {
  for (var key in obj) {
    if (obj.hasOwnProperty(key)) {
      if (callback(key, obj[key])) {
        break
      }
    }
  }
}

var palette = {
  bold: 1,
  faint: 2,
  red: 31,
  green: 32,
  cyan: 36,
  //TODO: add others?
}

var groupPrefix = '█'
var detailsPrefix = '↳'
var succMark = '✓'
var failMark = '✗'
var defaultOptions = {
  indent: ' ',
  enableColors: true,
  summaryTimeUnit: null,
  summaryTrendStats: null,
}

// strWidth tries to return the actual width the string will take up on the
// screen, without any terminal formatting, unicode ligatures, etc.
function strWidth(s) {
  // TODO: determine if NFC or NFKD are not more appropriate? or just give up? https://hsivonen.fi/string-length/
  var data = s.normalize('NFKC') // This used to be NFKD in Go, but this should be better
  var inEscSeq = false
  var inLongEscSeq = false
  var width = 0
  for (var char of data) {
    if (char.done) {
      break
    }

    // Skip over ANSI escape codes.
    if (char == '\x1b') {
      inEscSeq = true
      continue
    }
    if (inEscSeq && char == '[') {
      inLongEscSeq = true
      continue
    }
    if (inEscSeq && inLongEscSeq && char.charCodeAt(0) >= 0x40 && char.charCodeAt(0) <= 0x7e) {
      inEscSeq = false
      inLongEscSeq = false
      continue
    }
    if (inEscSeq && !inLongEscSeq && char.charCodeAt(0) >= 0x40 && char.charCodeAt(0) <= 0x5f) {
      inEscSeq = false
      continue
    }

    if (!inEscSeq && !inLongEscSeq) {
      width++
    }
  }
  return width
}

function summarizeCheck(indent, check, decorate) {
  if (check.fails == 0) {
    return decorate(indent + succMark + ' ' + check.name, palette.green)
  }

  var succPercent = Math.floor((100 * check.passes) / (check.passes + check.fails))
  return decorate(
    indent +
      failMark +
      ' ' +
      check.name +
      '\n' +
      indent +
      ' ' +
      detailsPrefix +
      '  ' +
      succPercent +
      '% — ' +
      succMark +
      ' ' +
      check.passes +
      ' / ' +
      failMark +
      ' ' +
      check.fails,
    palette.red
  )
}

function summarizeGroup(indent, group, decorate) {
  var result = []
  if (group.name != '') {
    result.push(indent + groupPrefix + ' ' + group.name + '\n')
    indent = indent + '  '
  }

  for (var i = 0; i < group.checks.length; i++) {
    result.push(summarizeCheck(indent, group.checks[i], decorate))
  }
  if (group.checks.length > 0) {
    result.push('')
  }
  for (var i = 0; i < group.groups.length; i++) {
    Array.prototype.push.apply(result, summarizeGroup(indent, group.groups[i], decorate))
  }

  return result
}

function displayNameForMetric(name) {
  var subMetricPos = name.indexOf('{')
  if (subMetricPos >= 0) {
    return '{ ' + name.substring(subMetricPos + 1, name.length - 1) + ' }'
  }
  return name
}

function indentForMetric(name) {
  if (name.indexOf('{') >= 0) {
    return '  '
  }
  return ''
}

function humanizeBytes(bytes) {
  var units = ['B', 'kB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
  var base = 1000
  if (bytes < 10) {
    return bytes + ' B'
  }

  var e = Math.floor(Math.log(bytes) / Math.log(base))
  var suffix = units[e | 0]
  var val = Math.floor((bytes / Math.pow(base, e)) * 10 + 0.5) / 10
  return val.toFixed(val < 10 ? 1 : 0) + ' ' + suffix
}

var unitMap = {
  s: { unit: 's', coef: 0.001 },
  ms: { unit: 'ms', coef: 1 },
  us: { unit: 'µs', coef: 1000 },
}

function toFixedNoTrailingZeros(val, prec) {
  // TODO: figure out something better?
  return parseFloat(val.toFixed(prec)).toString()
}

function toFixedNoTrailingZerosTrunc(val, prec) {
  var mult = Math.pow(10, prec)
  return toFixedNoTrailingZeros(Math.trunc(mult * val) / mult, prec)
}

function humanizeGenericDuration(dur) {
  if (dur === 0) {
    return '0s'
  }

  if (dur < 0.001) {
    // smaller than a microsecond, print nanoseconds
    return Math.trunc(dur * 1000000) + 'ns'
  }
  if (dur < 1) {
    // smaller than a millisecond, print microseconds
    return toFixedNoTrailingZerosTrunc(dur * 1000, 2) + 'µs'
  }
  if (dur < 1000) {
    // duration is smaller than a second
    return toFixedNoTrailingZerosTrunc(dur, 2) + 'ms'
  }

  var result = toFixedNoTrailingZerosTrunc((dur % 60000) / 1000, dur > 60000 ? 0 : 2) + 's'
  var rem = Math.trunc(dur / 60000)
  if (rem < 1) {
    // less than a minute
    return result
  }
  result = (rem % 60) + 'm' + result
  rem = Math.trunc(rem / 60)
  if (rem < 1) {
    // less than an hour
    return result
  }
  return rem + 'h' + result
}

function humanizeDuration(dur, timeUnit) {
  if (timeUnit !== '' && unitMap.hasOwnProperty(timeUnit)) {
    return (dur * unitMap[timeUnit].coef).toFixed(2) + unitMap[timeUnit].unit
  }

  return humanizeGenericDuration(dur)
}

function humanizeValue(val, metric, timeUnit) {
  if (metric.type == 'rate') {
    // Truncate instead of round when decreasing precision to 2 decimal places
    return (Math.trunc(val * 100 * 100) / 100).toFixed(2) + '%'
  }

  switch (metric.contains) {
    case 'data':
      return humanizeBytes(val)
    case 'time':
      return humanizeDuration(val, timeUnit)
    default:
      return toFixedNoTrailingZeros(val, 6)
  }
}

function nonTrendMetricValueForSum(metric, timeUnit) {
  switch (metric.type) {
    case 'counter':
      return [
        humanizeValue(metric.values.count, metric, timeUnit),
        humanizeValue(metric.values.rate, metric, timeUnit) + '/s',
      ]
    case 'gauge':
      return [
        humanizeValue(metric.values.value, metric, timeUnit),
        'min=' + humanizeValue(metric.values.min, metric, timeUnit),
        'max=' + humanizeValue(metric.values.max, metric, timeUnit),
      ]
    case 'rate':
      return [
        humanizeValue(metric.values.rate, metric, timeUnit),
        succMark + ' ' + metric.values.passes,
        failMark + ' ' + metric.values.fails,
      ]
    default:
      return ['[no data]']
  }
}

function summarizeMetrics(options, data, decorate) {
  var indent = options.indent + '  '
  var result = []

  var names = []
  var nameLenMax = 0

  var nonTrendValues = {}
  var nonTrendValueMaxLen = 0
  var nonTrendExtras = {}
  var nonTrendExtraMaxLens = [0, 0]

  var trendCols = {}
  var numTrendColumns = options.summaryTrendStats.length
  var trendColMaxLens = new Array(numTrendColumns).fill(0)
  forEach(data.metrics, function (name, metric) {
    names.push(name)
    // When calculating widths for metrics, account for the indentation on submetrics.
    var displayName = indentForMetric(name) + displayNameForMetric(name)
    var displayNameWidth = strWidth(displayName)
    if (displayNameWidth > nameLenMax) {
      nameLenMax = displayNameWidth
    }

    if (metric.type == 'trend') {
      var cols = []
      for (var i = 0; i < numTrendColumns; i++) {
        var tc = options.summaryTrendStats[i]
        var value = metric.values[tc]
        if (tc === 'count') {
          value = value.toString()
        } else {
          value = humanizeValue(value, metric, options.summaryTimeUnit)
        }
        var valLen = strWidth(value)
        if (valLen > trendColMaxLens[i]) {
          trendColMaxLens[i] = valLen
        }
        cols[i] = value
      }
      trendCols[name] = cols
      return
    }
    var values = nonTrendMetricValueForSum(metric, options.summaryTimeUnit)
    nonTrendValues[name] = values[0]
    var valueLen = strWidth(values[0])
    if (valueLen > nonTrendValueMaxLen) {
      nonTrendValueMaxLen = valueLen
    }
    nonTrendExtras[name] = values.slice(1)
    for (var i = 1; i < values.length; i++) {
      var extraLen = strWidth(values[i])
      if (extraLen > nonTrendExtraMaxLens[i - 1]) {
        nonTrendExtraMaxLens[i - 1] = extraLen
      }
    }
  })

  // sort all metrics but keep sub metrics grouped with their parent metrics
  names.sort(function (metric1, metric2) {
    var parent1 = metric1.split('{', 1)[0]
    var parent2 = metric2.split('{', 1)[0]
    var result = parent1.localeCompare(parent2)
    if (result !== 0) {
      return result
    }
    var sub1 = metric1.substring(parent1.length)
    var sub2 = metric2.substring(parent2.length)
    return sub1.localeCompare(sub2)
  })

  var getData = function (name) {
    if (trendCols.hasOwnProperty(name)) {
      var cols = trendCols[name]
      var tmpCols = new Array(numTrendColumns)
      for (var i = 0; i < cols.length; i++) {
        tmpCols[i] =
          options.summaryTrendStats[i] +
          '=' +
          decorate(cols[i], palette.cyan) +
          ' '.repeat(trendColMaxLens[i] - strWidth(cols[i]))
      }
      return tmpCols.join(' ')
    }

    var value = nonTrendValues[name]
    var fmtData = decorate(value, palette.cyan) + ' '.repeat(nonTrendValueMaxLen - strWidth(value))

    var extras = nonTrendExtras[name]
    if (extras.length == 1) {
      fmtData = fmtData + ' ' + decorate(extras[0], palette.cyan, palette.faint)
    } else if (extras.length > 1) {
      var parts = new Array(extras.length)
      for (var i = 0; i < extras.length; i++) {
        parts[i] =
          decorate(extras[i], palette.cyan, palette.faint) +
          ' '.repeat(nonTrendExtraMaxLens[i] - strWidth(extras[i]))
      }
      fmtData = fmtData + ' ' + parts.join(' ')
    }

    return fmtData
  }

  for (var name of names) {
    var metric = data.metrics[name]
    var mark = ' '
    var markColor = function (text) {
      return text
    } // noop

    if (metric.thresholds) {
      mark = succMark
      markColor = function (text) {
        return decorate(text, palette.green)
      }
      forEach(metric.thresholds, function (name, threshold) {
        if (!threshold.ok) {
          mark = failMark
          markColor = function (text) {
            return decorate(text, palette.red)
          }
          return true // break
        }
      })
    }
    var fmtIndent = indentForMetric(name)
    var fmtName = displayNameForMetric(name)
    fmtName =
      fmtName +
      decorate(
        '.'.repeat(nameLenMax - strWidth(fmtName) - strWidth(fmtIndent) + 3) + ':',
        palette.faint
      )

    result.push(indent + fmtIndent + markColor(mark) + ' ' + fmtName + ' ' + getData(name))
  }

  return result
}

function generateTextSummary(data, options) {
  var mergedOpts = Object.assign({}, defaultOptions, data.options, options)
  var lines = []

  // TODO: move all of these functions into an object with methods?
  var decorate = function (text) {
    return text
  }
  if (mergedOpts.enableColors) {
    decorate = function (text, color /*, ...rest*/) {
      var result = '\x1b[' + color
      for (var i = 2; i < arguments.length; i++) {
        result += ';' + arguments[i]
      }
      return result + 'm' + text + '\x1b[0m'
    }
  }

  Array.prototype.push.apply(
    lines,
    summarizeGroup(mergedOpts.indent + '    ', data.root_group, decorate)
  )

  Array.prototype.push.apply(lines, summarizeMetrics(mergedOpts, data, decorate))

  return lines.join('\n')
}

exports.humanizeValue = humanizeValue
exports.textSummary = generateTextSummary

var replacements = {
  '&': '&amp;',
  '<': '&lt;',
  '>': '&gt;',
  "'": '&#39;',
  '"': '&quot;',
}

function escapeHTML(str) {
  // TODO: something more robust?
  return str.replace(/[&<>'"]/g, function (char) {
    return replacements[char]
  })
}

function generateJUnitXML(data, options) {
  var failures = 0
  var cases = []

  forEach(data.metrics, function (metricName, metric) {
    if (!metric.thresholds) {
      return
    }
    forEach(metric.thresholds, function (thresholdName, threshold) {
      if (threshold.ok) {
        cases.push(
          '<testcase name="' + escapeHTML(metricName) + ' - ' + escapeHTML(thresholdName) + '" />'
        )
      } else {
        failures++
        cases.push(
          '<testcase name="' +
            escapeHTML(metricName) +
            ' - ' +
            escapeHTML(thresholdName) +
            '"><failure message="failed" /></testcase>'
        )
      }
    })
  })

  var name = options && options.name ? escapeHTML(options.name) : 'k6 thresholds'

  return (
    '<?xml version="1.0"?>\n<testsuites tests="' +
    cases.length +
    '" failures="' +
    failures +
    '">\n' +
    '<testsuite name="' +
    name +
    '" tests="' +
    cases.length +
    '" failures="' +
    failures +
    '">' +
    cases.join('\n') +
    '\n</testsuite >\n</testsuites >'
  )
}

exports.jUnit = generateJUnitXML
