`default_nettype none

module Uart(
    input wire logic reset_n,
    input wire logic clk,

    output wire logic tx,
    input wire logic rx,

    output logic has_errored);

    logic [7:0] write_data;
    logic write_req;
    logic ready;
    uart_transmitter uart_transmitter0(
        .reset_n(reset_n),
        .clk(clk),

        .write_data(write_data),
        .write_req(write_req),
        .ready(ready),

        .tx(tx));

    Lfsr tx_lfsr(
        .reset_n(reset_n),
        .clk(clk),

        .value(write_data),
        .shift_enable(write_req & ready));

    logic [7:0] read_data;
    logic read_data_ready;
    UartRx uart_rx(
        .reset_n(reset_n),
        .clk(clk),

        .rx(rx),

        .data(read_data),
        .data_ready(read_data_ready));

    logic [7:0] rx_lfsr_value;
    Lfsr rx_lfsr(
        .reset_n(reset_n),
        .clk(clk),

        .value(rx_lfsr_value),
        .shift_enable(read_data_ready));

    logic has_errored_next;

    always_comb begin
        has_errored_next = has_errored;

        if (read_data_ready) begin
            has_errored_next = read_data != rx_lfsr_value;
        end
    end

    always_ff @(posedge clk, negedge reset_n) begin
        if (~reset_n) begin
            has_errored <= 1'b0;
        end
        else begin
            has_errored <= has_errored_next;
        end
    end

    assign write_req = ~has_errored;

endmodule
