meta:
  id: ipfire_libloc_db_v1
  file-extension: db
  endian: be
  license: MIT/Apache-2.0
seq:
  - id: header
    type: header
instances:
  as:
    type: as
    pos: header.as.offset
    repeat: expr
    repeat-expr: header.as.length / 8 # 8 = sizeof(as)
  networks:
    type: network
    pos: header.networks.offset
    repeat: expr
    repeat-expr: header.networks.length / 12 # 12 = sizeof(network)
  network_nodes:
    type: network_node
    pos: header.network_nodes.offset
    repeat: expr
    repeat-expr: header.network_nodes.length / 12 # 12 = sizeof(network_node)
  countries:
    type: country
    pos: header.countries.offset
    repeat: expr
    repeat-expr: header.countries.length / 8 # 8 = sizeof(country)
  strings:
    type: strzseq
    pos: header.pool.offset
    size: header.pool.length
    repeat: eos
types:
  header:
    seq:
      - id: magic
        contents: 'LOCDBXX'
      - id: version
        contents: "\x01"
      - id: created_at
        type: u8
      - id: vendor
        type: str_ref
      - id: description
        type: str_ref
      - id: license
        type: str_ref
      - id: as
        type: file_range
      - id: networks
        type: file_range
      - id: network_nodes
        type: file_range
      - id: countries
        type: file_range
      - id: pool
        type: file_range
      - id: signature1_length
        type: u2
      - id: signature2_length
        type: u2
      - id: signature1
        size: signature1_length
      - id: signature1_padding
        size: 2048 - signature1_length
      - id: signature2
        size: signature2_length
      - id: signature2_padding
        size: 2048 - signature2_length
      - id: padding
        size: 32
  file_range:
    seq:
      - id: offset
        type: u4
      - id: length
        type: u4
  str_ref:
    seq:
      - id: offset
        type: u4
    instances:
      value:
        type: strz
        encoding: utf8
        io: _root._io
        pos: _root.header.pool.offset + offset
  network_node:
    seq:
      - id: child_zero
        type: u4
      - id: child_one
        type: u4
      - id: network
        type: u4
  network:
    seq:
      - id: country_code
        type: str
        encoding: ascii
        size: 2
      - id: padding
        size: 2
      - id: asn
        type: u4
      - id: flags
        type: u2
      - id: padding2
        size: 2
  as:
    seq:
      - id: number
        type: u4
      - id: name
        type: str_ref
  country:
    seq:
      - id: code
        type: str
        encoding: ascii
        size: 2
      - id: continent_code
        type: str
        encoding: ascii
        size: 2
      - id: name
        type: str_ref
  strzseq:
    seq:
      - id: str
        type: strz
        encoding: utf8
        repeat: eos
